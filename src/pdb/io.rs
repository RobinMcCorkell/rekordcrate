// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
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

use super::{DatabaseType, Header, Page, PageIndex};
use crate::{
    pdb::PageType,
    util::{RekordcrateError, RekordcrateResult, TableIndex},
};
use binrw::{
    binrw,
    io::{Seek, SeekFrom, Write},
    BinRead, BinResult, BinWrite, Endian,
};
use std::io::Read;

/// A lazily loaded PDB database.
#[binrw]
#[brw(little)]
#[br(import(db_type: DatabaseType))]
#[derive(Debug, PartialEq)]
struct LazyDatabase {
    /// The PDB header.
    #[br(args(db_type))]
    header: Header,
    /// The pages of the database, initially not loaded.
    #[br(calc = vec![LazyPage::NotLoaded; (header.next_unused_page.0 - 1) as usize])]
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
        _writer: &mut W,
        _endian: Endian,
        (_page_size,): Self::Args<'_>,
    ) -> BinResult<()> {
        todo!()
    }
}

/// A PDB file opened for reading or writing.
pub struct PdbFile<'io, IO> {
    io: &'io mut IO,
    db_type: DatabaseType,
    content: LazyDatabase,
}

impl<'io, IO> std::fmt::Debug for PdbFile<'io, IO> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdbFile")
            .field("db_type", &self.db_type)
            .field("header", &self.content.header)
            .finish()
    }
}

impl<'r, R: Read + Seek> PdbFile<'r, R> {
    /// Opens a PDB file without writing back to disk.
    /// Still allows modifying data in memory.
    pub fn open_non_persistent(io: &'r mut R, db_type: DatabaseType) -> RekordcrateResult<Self> {
        let endian = Endian::Little;
        let content = LazyDatabase::read_options(io, endian, (db_type,))?;
        Ok(Self {
            io,
            db_type,
            content,
        })
    }

    /// Loads a page into memory.
    pub fn load_page(&mut self, index: PageIndex) -> RekordcrateResult<&mut Page> {
        let endian = Endian::Little;
        let page_entry = self
            .content
            .pages
            .get_mut(index.0 as usize - 1)
            .ok_or_else(|| RekordcrateError::PageNotPresent(index))?;
        if let LazyPage::NotLoaded = page_entry {
            // Load the page from the file
            let page_offset = SeekFrom::Start(index.offset(self.content.header.page_size));
            self.io.seek(page_offset).map_err(binrw::Error::Io)?;
            let page = Page::read_options(
                self.io,
                endian,
                (self.content.header.page_size, self.db_type),
            )?;
            *page_entry = LazyPage::Loaded(page);
        }
        match page_entry {
            LazyPage::Loaded(page) => Ok(page),
            _ => unreachable!(),
        }
    }

    /// Loads all pages for a table into memory.
    pub fn load_pages_for_table<'pdb>(
        &'pdb mut self,
        table_index: TableIndex,
    ) -> RekordcrateResult<PageIterator<'pdb, 'r, R>> {
        let table = self
            .get_header()
            .tables
            .get(table_index.0)
            .ok_or_else(|| RekordcrateError::TableNotPresent(table_index))?;
        let (first, last) = (table.first_page, table.last_page);

        Ok(PageIterator {
            pdb_file: self,
            next: Some(first),
            last,
        })
    }

    /// Loads all pages for a page type into memory.
    pub fn load_pages<'pdb>(
        &'pdb mut self,
        page_type: PageType,
    ) -> RekordcrateResult<PageIterator<'pdb, 'r, R>> {
        let (_, table) = self
            .get_header()
            .find_table(page_type)
            .ok_or_else(|| RekordcrateError::TableTypeNotPresent(page_type))?;
        let (first, last) = (table.first_page, table.last_page);

        Ok(PageIterator {
            pdb_file: self,
            next: Some(first),
            last,
        })
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

/// An "iterator" over pages in a PDB file.
///
/// This isn't actually an `Iterator` because we cannot
/// lend items with the current Rust `Iterator` trait.
#[derive(Debug)]
pub struct PageIterator<'pdb, 'io, IO> {
    pdb_file: &'pdb mut PdbFile<'io, IO>,
    next: Option<PageIndex>,
    last: PageIndex,
}

impl<'pdb, 'io, IO> PageIterator<'pdb, 'io, IO>
where
    IO: Read + Seek,
{
    /// Loads the next page in the iterator.
    ///
    /// Behaves almost like `Iterator::next` but the item is
    /// wrapped in a `RekordcrateResult` to capture I/O errors.
    pub fn load_next(&mut self) -> RekordcrateResult<Option<&mut Page>> {
        match self.next {
            None => Ok(None),
            Some(page_index) => {
                let page = self.pdb_file.load_page(page_index)?;
                if page_index == self.last {
                    self.next = None;
                } else {
                    self.next = Some(page.header.next_page);
                }
                Ok(Some(page))
            }
        }
    }

    /// Maps each page in the iterator using the given function.
    ///
    /// Behaves almost like `Iterator::map` but the iterated `T` is
    /// wrapped in a `RekordcrateResult` to capture I/O errors.
    pub fn map<F, T>(
        mut self,
        mut f: F,
    ) -> impl Iterator<Item = RekordcrateResult<T>> + use<'pdb, 'io, IO, F, T>
    where
        F: FnMut(&mut Page) -> T,
    {
        std::iter::from_fn(move || match self.load_next() {
            Ok(Some(page)) => Some(Ok(f(page))),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Maps and filters each page in the iterator using the given function.
    ///
    /// Behaves almost like `Iterator::filter_map` but the iterated `T` is
    /// wrapped in a `RekordcrateResult` to capture I/O errors.
    pub fn filter_map<F, T>(
        self,
        f: F,
    ) -> impl Iterator<Item = RekordcrateResult<T>> + use<'pdb, 'io, IO, F, T>
    where
        F: FnMut(&mut Page) -> Option<T>,
    {
        self.map(f).filter_map(Result::transpose)
    }
}
