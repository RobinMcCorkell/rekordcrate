// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Parser for Pioneer DeviceSQL database exports (PDB).
//!
//! The Rekordbox DJ software uses writes PDB files to `/PIONEER/rekordbox/export.pdb`.
//!
//! Most of the file format has been reverse-engineered by Henry Betts, Fabian Lesniak and James
//! Elliott.
//!
//! - <https://github.com/Deep-Symmetry/crate-digger/blob/master/doc/Analysis.pdf>
//! - <https://djl-analysis.deepsymmetry.org/rekordbox-export-analysis/exports.html>
//! - <https://github.com/henrybetts/Rekordbox-Decoding>
//! - <https://github.com/flesniak/python-prodj-link/tree/master/prodj/pdblib>

use super::*;
use crate::util::{RekordcrateError, RekordcrateResult, TableIndex};
use binrw::{binrw, io::SeekFrom, BinRead, BinResult, BinWrite, Endian};
use fallible_iterator::{FallibleIterator, IteratorExt};
use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};

/// A lazily loaded PDB database.
#[binrw]
#[brw(little)]
#[br(import(db_type: DatabaseType))]
#[derive(Debug, PartialEq)]
struct LazyDatabase {
    /// The PDB header.
    #[br(args(db_type))]
    #[bw(pad_size_to = header.page_size as usize)]
    header: Header,
    /// The pages of the database, initially not loaded.
    #[br(calc = vec![LazyPage::NotLoaded; header.next_unused_page.0.saturating_sub(1) as usize])]
    #[bw(args(header.page_size))]
    pages: Vec<LazyPage>,
}

#[derive(Debug, PartialEq, Clone)]
enum LazyPage {
    NotLoaded,
    Loaded(Page),
}

impl BinWrite for LazyPage {
    type Args<'a> = (u32,);

    fn write_options<W: Write + Seek>(
        &self,
        writer: &mut W,
        endian: Endian,
        (page_size,): Self::Args<'_>,
    ) -> BinResult<()> {
        match self {
            LazyPage::NotLoaded => {
                // Just seek forward without writing anything.
                writer.seek(SeekFrom::Current(page_size as i64))?;
                Ok(())
            }
            LazyPage::Loaded(page) => page.write_options(writer, endian, (page_size,)),
        }
    }
}

fn read_page<IO: Read + Seek>(
    io: &mut IO,
    page_index: PageIndex,
    page_size: u32,
    db_type: DatabaseType,
) -> RekordcrateResult<Page> {
    let endian = Endian::Little;
    let page_offset = SeekFrom::Start(page_index.offset(page_size));
    io.seek(page_offset).map_err(binrw::Error::Io)?;
    let page = Page::read_options(io, endian, (page_size, db_type))?;
    Ok(page)
}

/// A PDB database opened for reading or writing.
#[derive(Debug)]
pub struct Database<IO> {
    io: IO,
    db_type: DatabaseType,
    content: LazyDatabase,
}

impl<R: Read + Seek> Database<R> {
    /// Opens a PDB database without writing back to disk.
    /// Still allows modifying data in memory.
    pub fn open_non_persistent(mut io: R, db_type: DatabaseType) -> RekordcrateResult<Self> {
        let endian = Endian::Little;
        let content = LazyDatabase::read_options(&mut io, endian, (db_type,))?;
        Ok(Self {
            io,
            db_type,
            content,
        })
    }

    /// Loads a page into memory.
    pub fn load_page(&mut self, index: PageIndex) -> RekordcrateResult<&mut Page> {
        let page_entry = self
            .content
            .pages
            .get_mut(index.0 as usize - 1)
            .ok_or_else(|| RekordcrateError::PageNotPresent(index))?;
        if let LazyPage::NotLoaded = page_entry {
            let page = read_page(
                &mut self.io,
                index,
                self.content.header.page_size,
                self.db_type,
            )?;
            *page_entry = LazyPage::Loaded(page);
        }
        match page_entry {
            LazyPage::Loaded(page) => Ok(page),
            _ => unreachable!(),
        }
    }

    /// Loads all pages for a table into memory and iterates over them.
    pub fn iter_pages_for_table<'db>(
        &'db mut self,
        table_index: TableIndex,
    ) -> RekordcrateResult<PageIterator<'db, R>> {
        let table = self
            .get_header()
            .tables
            .get(table_index.0)
            .ok_or_else(|| RekordcrateError::TableNotPresent(table_index))?;
        let (first_page, last_page) = (table.first_page, table.last_page);

        Ok(PageIterator {
            db_pages: self.content.pages.as_mut_slice(),
            db_pages_offset: 1, // Page indices are 1-based, so the first page is at offset 0 in the slice.
            db_io: &mut self.io,
            db_page_size: self.content.header.page_size,
            db_type: self.db_type,
            next_page: Some(first_page),
            last_page,
        })
    }

    /// Loads all pages for a page type into memory and iterates over them.
    pub fn iter_pages(&mut self, page_type: PageType) -> RekordcrateResult<PageIterator<'_, R>> {
        let (_, table) = self
            .get_header()
            .find_table(page_type)
            .ok_or_else(|| RekordcrateError::TableTypeNotPresent(page_type))?;
        let (first_page, last_page) = (table.first_page, table.last_page);

        Ok(PageIterator {
            db_pages: self.content.pages.as_mut_slice(),
            db_pages_offset: 1, // Page indices are 1-based, so the first page is at offset 0 in the slice.
            db_io: &mut self.io,
            db_page_size: self.content.header.page_size,
            db_type: self.db_type,
            next_page: Some(first_page),
            last_page,
        })
    }

    /// Loads all pages for a page type into memory and iterates over their data rows.
    pub fn iter_rows<'a, RowT: RowVariant + 'a>(
        &'a mut self,
    ) -> RekordcrateResult<impl FallibleIterator<Item = &'a mut RowT, Error = RekordcrateError>>
    {
        Ok(self
            .iter_pages(RowT::PAGE_TYPE)?
            .filter_map(|page| Ok(page.content.as_data_mut()))
            .flat_map(|dpc| {
                Ok(dpc
                    .rows
                    .values_mut()
                    .into_fallible()
                    .map_err(|_: core::convert::Infallible| unreachable!()))
            })
            // The parsed row type is determined from the page type, so if we find an unexpected
            // variant then there is a code bug (not simply a corrupt DB).
            .map(|row| Ok(row.as_variant_mut().expect("unexpected row type"))))
    }

    /// Returns the total size of the underlying IO in bytes.
    ///
    /// Saves and restores the current stream position.
    pub fn file_size(&mut self) -> Result<u64, std::io::Error> {
        let pos = self.io.seek(SeekFrom::Current(0))?;
        let size = self.io.seek(SeekFrom::End(0))?;
        self.io.seek(SeekFrom::Start(pos))?;
        Ok(size)
    }

    /// Returns the database type.
    #[must_use]
    pub fn db_type(&self) -> DatabaseType {
        self.db_type
    }

    /// Returns a reference to the PDB header.
    #[must_use]
    pub fn get_header(&self) -> &Header {
        &self.content.header
    }

    /// Returns a mutable reference to the PDB header.
    #[must_use]
    pub fn get_header_mut(&mut self) -> &mut Header {
        &mut self.content.header
    }
}

impl<RW: Read + Write + Seek> Database<RW> {
    /// Opens a PDB database for reading and writing.
    pub fn open(mut io: RW, db_type: DatabaseType) -> RekordcrateResult<Self> {
        let endian = Endian::Little;
        let content = LazyDatabase::read_options(&mut io, endian, (db_type,))?;
        Ok(Self {
            io,
            db_type,
            content,
        })
    }

    /// Creates a new PDB database with a blank set of tables.
    pub fn create(io: RW, db_type: DatabaseType) -> RekordcrateResult<Self> {
        const PAGE_SIZE: u32 = 4096;
        const NUM_TABLES: u32 = 20;

        let page_types: [PageType; 20] = [
            PageType::Plain(PlainPageType::Tracks),
            PageType::Plain(PlainPageType::Genres),
            PageType::Plain(PlainPageType::Artists),
            PageType::Plain(PlainPageType::Albums),
            PageType::Plain(PlainPageType::Labels),
            PageType::Plain(PlainPageType::Keys),
            PageType::Plain(PlainPageType::Colors),
            PageType::Plain(PlainPageType::PlaylistTree),
            PageType::Plain(PlainPageType::PlaylistEntries),
            PageType::Unknown(9),
            PageType::Unknown(10),
            PageType::Plain(PlainPageType::HistoryPlaylists),
            PageType::Plain(PlainPageType::HistoryEntries),
            PageType::Plain(PlainPageType::Artwork),
            PageType::Unknown(14),
            PageType::Unknown(15),
            PageType::Plain(PlainPageType::Columns),
            PageType::Plain(PlainPageType::Menu),
            PageType::Unknown(18),
            PageType::Plain(PlainPageType::History),
        ];

        // Each table gets two pages: an index page (odd-numbered) followed by a data page
        // (even-numbered). `next_unused_page` points past the last page (1..=2*NUM_TABLES).
        let next_unused_page = PageIndex(NUM_TABLES * 2 + 1);

        let tables: Vec<Table> = page_types
            .iter()
            .enumerate()
            .map(|(i, &page_type)| {
                let index_page = PageIndex(i as u32 * 2 + 1);
                let data_page = PageIndex(i as u32 * 2 + 2);
                Table {
                    page_type,
                    empty_candidate: data_page.0,
                    first_page: index_page,
                    last_page: index_page,
                }
            })
            .collect();

        let header = Header {
            page_size: PAGE_SIZE,
            num_tables: NUM_TABLES,
            next_unused_page,
            unknown: 5,
            sequence: 1,
            tables,
        };

        let free_size = DataPageContent::page_heap_size(PAGE_SIZE) as u16;
        let mut pages: Vec<LazyPage> = Vec::with_capacity(NUM_TABLES as usize * 2);

        for (i, &page_type) in page_types.iter().enumerate() {
            let index_page_idx = PageIndex(i as u32 * 2 + 1);
            let data_page_idx = PageIndex(i as u32 * 2 + 2);

            // Index page for this table. CDJ hardware requires index pages to be present
            // at the start of each table's page chain.
            pages.push(LazyPage::Loaded(Page {
                header: PageHeader {
                    page_index: index_page_idx,
                    page_type,
                    next_page: data_page_idx,
                    unknown1: 1,
                    unknown2: 0,
                    packed_row_counts: PackedRowCounts::default(),
                    page_flags: PageFlags::new_index_page(),
                    free_size: 0,
                    used_size: 0,
                },
                content: PageContent::Index(IndexPageContent {
                    header: IndexPageHeader {
                        unknown_a: 0x1FFF,
                        unknown_b: 0x1FFF,
                        next_offset: 0,
                        page_index: index_page_idx,
                        // Null sentinel for an empty-table index page.
                        next_page: PageIndex(0x03FF_FFFF),
                        num_entries: 0,
                        first_empty: 0x1FFF,
                    },
                    entries: vec![],
                }),
            }));

            // Data page for this table (initially empty).
            pages.push(LazyPage::Loaded(Page {
                header: PageHeader {
                    page_index: data_page_idx,
                    page_type,
                    next_page: next_unused_page,
                    unknown1: 0,
                    unknown2: 0,
                    packed_row_counts: PackedRowCounts::default(),
                    page_flags: PageFlags::new_data_page(),
                    free_size,
                    used_size: 0,
                },
                content: PageContent::Data(DataPageContent {
                    header: DataPageHeader {
                        unknown5: 0,
                        unknown_not_num_rows_large: 0,
                        unknown6: 0,
                        unknown7: 0,
                    },
                    row_groups: vec![],
                    rows: BTreeMap::new(),
                }),
            }));
        }

        let mut db = Self {
            io,
            db_type,
            content: LazyDatabase { header, pages },
        };
        db.flush()?;
        Ok(db)
    }

    /// Promotes the empty-candidate page for a table to a real data page and allocates a
    /// fresh empty-candidate after it.
    ///
    /// After promotion:
    /// - `promoted_page.header.next_page` points to the new empty-candidate
    /// - `table.last_page` is updated to the promoted page index
    /// - `table.empty_candidate` is updated to the new empty-candidate index
    /// - `header.next_unused_page` is advanced past the new empty-candidate
    fn promote_ec_to_data_page(
        &mut self,
        page_type: PageType,
        ec_idx: PageIndex,
    ) -> RekordcrateResult<()> {
        let page_size = self.content.header.page_size;
        let new_ec_idx = self.content.header.next_unused_page;
        let after_new_ec = PageIndex(new_ec_idx.0 + 1);
        let free_size = DataPageContent::page_heap_size(page_size) as u16;

        // Ensure the EC page is in memory, then point it at the new EC.
        let ec_slice_idx = ec_idx.0 as usize - 1;
        let needs_load = matches!(
            self.content.pages.get(ec_slice_idx),
            Some(LazyPage::NotLoaded)
        );
        if needs_load {
            let mut page = read_page(&mut self.io, ec_idx, page_size, self.db_type)?;
            page.header.next_page = new_ec_idx;
            self.content.pages[ec_slice_idx] = LazyPage::Loaded(page);
        } else if let Some(LazyPage::Loaded(page)) = self.content.pages.get_mut(ec_slice_idx) {
            page.header.next_page = new_ec_idx;
        }

        // Update table metadata.
        let (_, table) = self.content.header.find_table_mut(page_type).unwrap();
        table.last_page = ec_idx;
        table.empty_candidate = new_ec_idx.0;
        self.content.header.next_unused_page = after_new_ec;

        // Allocate the new empty-candidate page.
        self.content.pages.push(LazyPage::Loaded(Page {
            header: PageHeader {
                page_index: new_ec_idx,
                page_type,
                next_page: after_new_ec,
                unknown1: 0,
                unknown2: 0,
                packed_row_counts: PackedRowCounts::default(),
                page_flags: PageFlags::new_data_page(),
                free_size,
                used_size: 0,
            },
            content: PageContent::Data(DataPageContent {
                header: DataPageHeader {
                    unknown5: 0,
                    unknown_not_num_rows_large: 0,
                    unknown6: 0,
                    unknown7: 0,
                },
                row_groups: vec![],
                rows: BTreeMap::new(),
            }),
        }));

        Ok(())
    }

    /// Inserts a row into the appropriate table based on its type.
    ///
    /// If the current last page for the row's table is full, a new page is allocated.
    pub fn insert_row(&mut self, row: Row) -> RekordcrateResult<()> {
        let page_type = row.page_type();
        let bytes = row.heap_bytes_required(());
        let page_size = self.content.header.page_size;

        // Get the current table state.
        let (first_page, last_page, empty_candidate) = {
            let table = self
                .content
                .header
                .find_table(page_type)
                .ok_or_else(|| RekordcrateError::TableTypeNotPresent(page_type))?
                .1;
            (table.first_page, table.last_page, table.empty_candidate)
        };

        // If the table is empty (last_page == first_page = index-only state), promote the
        // pre-allocated empty_candidate to be the first real data page and reserve a new one.
        if last_page == first_page {
            self.promote_ec_to_data_page(page_type, PageIndex(empty_candidate))?;
        }

        // Get the last page index for this table type (now guaranteed to be a data page).
        let last_page_idx = self
            .content
            .header
            .find_table(page_type)
            .unwrap()
            .1
            .last_page;

        let slice_index = last_page_idx.0 as usize - 1;

        // Load the page if not already loaded.
        if matches!(self.content.pages[slice_index], LazyPage::NotLoaded) {
            let page = read_page(&mut self.io, last_page_idx, page_size, self.db_type)?;
            self.content.pages[slice_index] = LazyPage::Loaded(page);
        }

        // Try to allocate a row in the current last page.
        if let LazyPage::Loaded(page) = &mut self.content.pages[slice_index] {
            if let Some(insert) = page.allocate_row(bytes) {
                insert(row);
                return Ok(());
            }
        }

        // The page is full — promote the empty_candidate as the next data page and link
        // the old last page to it.
        let empty_candidate = self
            .content
            .header
            .find_table(page_type)
            .unwrap()
            .1
            .empty_candidate;
        self.promote_ec_to_data_page(page_type, PageIndex(empty_candidate))?;

        let new_last_page_idx = self
            .content
            .header
            .find_table(page_type)
            .unwrap()
            .1
            .last_page;

        // Link the old last page to the newly promoted data page.
        if let LazyPage::Loaded(old_page) = &mut self.content.pages[slice_index] {
            old_page.header.next_page = new_last_page_idx;
        }

        // Insert the row into the new last page.
        let new_slice_index = new_last_page_idx.0 as usize - 1;
        if let LazyPage::Loaded(page) = &mut self.content.pages[new_slice_index] {
            let insert = page
                .allocate_row(bytes)
                .expect("new empty page should have space for row");
            insert(row);
        }

        Ok(())
    }

    /// Flushes all changes to the underlying IO.
    pub fn flush(&mut self) -> RekordcrateResult<()> {
        let endian = Endian::Little;
        self.io.seek(SeekFrom::Start(0))?;
        self.content.write_options(&mut self.io, endian, ())?;
        Ok(())
    }

    /// Closes the database, flushing changes.
    pub fn close(mut self) -> RekordcrateResult<()> {
        self.flush()?;
        Ok(())
    }
}

/// An iterator over pages in a PDB database.
///
/// We use `FallibleIterator` instead of the standard `Iterator` trait
/// to improve the ergonomics of error handling while loading pages.
///
/// # Usage
///
/// ```no_run
/// # use rekordcrate::pdb::*;
/// # use rekordcrate::util::RekordcrateError;
/// # use rekordcrate::pdb::io::Database;
/// use fallible_iterator::FallibleIterator;
///
/// # let mut db: Database<std::fs::File> = unimplemented!();
/// // Loop over pages.
/// let mut page_iter = db.iter_pages(PageType::Plain(PlainPageType::Tracks))?;
/// while let Some(page) = page_iter.next()? {
///     // Process the page
/// }
///
/// // Iterate over pages using typical functional combinators.
/// // Note that combinators like `map` should return a `Result`.
/// let results: Vec<_> = db
///     .iter_pages(PageType::Plain(PlainPageType::Tracks))?
///     .map(|page| Ok(todo!()))
///     .collect()?;
/// # Ok::<(), RekordcrateError>(())
/// ```
#[derive(Debug)]
pub struct PageIterator<'db, IO> {
    db_pages: &'db mut [LazyPage],
    db_pages_offset: usize,
    db_io: &'db mut IO,
    db_page_size: u32,
    db_type: DatabaseType,

    next_page: Option<PageIndex>,
    last_page: PageIndex,
}

impl<'db, R: Read + Seek> FallibleIterator for PageIterator<'db, R> {
    type Item = &'db mut Page;
    type Error = RekordcrateError;

    /// Loads the next page in the iterator.
    fn next(&mut self) -> RekordcrateResult<Option<&'db mut Page>> {
        match self.next_page {
            None => Ok(None),
            Some(page_index) => {
                // Throw away references to pages lower than the next page index,
                // leaving our target page at the start of `pages`.
                // ASSUMPTION: pages in a table are linked in increasing order by index.
                let slice_index = (page_index.0 as usize)
                    .checked_sub(self.db_pages_offset)
                    .ok_or(RekordcrateError::PageOrderViolation(page_index))?;
                let db_pages: &'db mut [LazyPage] = std::mem::take(&mut self.db_pages);
                let (_, pages): (_, &'db mut [LazyPage]) = db_pages
                    .split_at_mut_checked(slice_index)
                    .ok_or(RekordcrateError::PageNotPresent(page_index))?;
                // Pull out the target page and leave the rest in `self.db_pages`.
                let (page_entry, pages): (&'db mut LazyPage, &'db mut [LazyPage]) = pages
                    .split_first_mut()
                    .ok_or(RekordcrateError::PageNotPresent(page_index))?;
                self.db_pages = pages;
                self.db_pages_offset = page_index.0 as usize + 1;

                if let LazyPage::NotLoaded = page_entry {
                    let page = read_page(self.db_io, page_index, self.db_page_size, self.db_type)?;
                    *page_entry = LazyPage::Loaded(page);
                }
                let page: &'db mut Page = match page_entry {
                    LazyPage::Loaded(page) => page,
                    _ => unreachable!(),
                };

                if page_index == self.last_page {
                    self.next_page = None;
                } else {
                    self.next_page = Some(page.header.next_page);
                }
                Ok(Some(page))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_pageiterator_safety() {
        // This was written when PageIterator used unsafe.
        // It's a small test and provides value in case we ever want to use unsafe again.
        // Run with `MIRIFLAGS="-Zmiri-disable-isolation" cargo +nightly miri test test_pageiterator_safety`.
        let file = File::open("data/pdb/num_rows/export.pdb").unwrap();
        let mut db = Database::open_non_persistent(file, DatabaseType::Plain).unwrap();
        let mut iter = db
            .iter_pages(PageType::Plain(PlainPageType::Tracks))
            .unwrap();

        let first = iter.next().unwrap().unwrap();
        let second = iter.next().unwrap().unwrap();

        // Should be disallowed since `db` is still borrowed by `iter` until all pages go out of scope.
        // let _iter2 = db
        //     .iter_pages(PageType::Plain(PlainPageType::Tracks))
        //     .unwrap();

        assert_eq!(
            first.header.page_type,
            PageType::Plain(PlainPageType::Tracks)
        );
        assert_eq!(
            second.header.page_type,
            PageType::Plain(PlainPageType::Tracks)
        );

        // Should be allowed since the `db` borrow can now be released.
        let _iter3 = db
            .iter_pages(PageType::Plain(PlainPageType::Tracks))
            .unwrap();
    }
}
