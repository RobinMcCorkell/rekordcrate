// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Structural validation of PDB database files.
//!
//! The [`validate`] function checks a set of invariants derived empirically from
//! rekordbox-generated database files. These invariants cover the file-level structure
//! (file size, page count), per-page fields (`page_index`, `page_flags`, `unknown2`),
//! table chain pointers (`next_page`, `empty_candidate`), and consistency between the
//! `PageHeader` and `FreeSpacePageHeader` page-index fields.
//!
//! # Usage
//!
//! ```no_run
//! # use rekordcrate::pdb::io::Database;
//! # use rekordcrate::pdb::validation::validate;
//! # use rekordcrate::pdb::DatabaseType;
//! # use std::fs::File;
//! let file = File::open("export.pdb").unwrap();
//! let mut db = Database::open_non_persistent(file, DatabaseType::Plain).unwrap();
//! let errors = validate(&mut db);
//! for e in &errors {
//!     eprintln!("{e}");
//! }
//! ```

use super::io::Database;
use super::*;
use std::collections::HashSet;
use std::io::{Read, Seek};
use thiserror::Error;

/// The page size expected in all known rekordbox exports.
const EXPECTED_PAGE_SIZE: u32 = 4096;
/// The sentinel value stored in `FreeSpacePageHeader.next_page` for an empty table.
const NEXT_PAGE_SENTINEL: u32 = 0x03FF_FFFF;
/// Expected number of tables in a `DatabaseType::Plain` (`export.pdb`) file.
const EXPECTED_NUM_TABLES_PLAIN: u32 = 20;
/// Expected number of tables in a `DatabaseType::Ext` (`exportExt.pdb`) file.
const EXPECTED_NUM_TABLES_EXT: u32 = 9;

/// A structural invariant violation found in a PDB database.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum ValidationError {
    /// File size is not a multiple of the declared page size.
    #[error("File size {file_size} is not a multiple of page size {page_size}")]
    FileSizeNotMultipleOfPageSize { file_size: u64, page_size: u32 },

    /// The declared page size is not the expected 4096 bytes.
    #[error("Unexpected page size: expected {expected}, found {actual}")]
    UnexpectedPageSize { expected: u32, actual: u32 },

    /// The database next-page sequence number is zero; it must be >= 1.
    #[error("Header next_page_sequence must be >= 1, got {0}")]
    SequenceZero(u32),

    /// The number of tables does not match the expected count for the database type.
    #[error("Expected {expected} tables for this database type, found {actual}")]
    WrongNumTables { expected: u32, actual: u32 },

    /// A non-EC page's `PageHeader.page_index` does not match its physical position.
    #[error(
        "Non-EC page at physical position {physical_position} has PageHeader.page_index = {page_index:?}"
    )]
    PageIndexMismatch {
        page_index: PageIndex,
        physical_position: u32,
    },

    /// A page's `PageHeader.unknown2` field is non-zero (always expected to be zero).
    #[error("Page {page_index:?} has non-zero PageHeader.unknown2: {value:#010x}")]
    PageUnknown2NonZero { page_index: PageIndex, value: u32 },

    /// A page has unexpected `page_flags` (must be 0x24, 0x34 for data or 0x64 for free-space).
    #[error("Page {page_index:?} has unexpected page_flags: {flags:#04x}")]
    InvalidPageFlags { page_index: PageIndex, flags: u8 },

    /// A table's `empty_candidate` is out of range (must be < `next_unused_page`).
    #[error(
        "Table {table_index}: empty_candidate {empty_candidate} >= next_unused_page {next_unused_page}"
    )]
    EmptyCandidateOutOfBounds {
        table_index: usize,
        empty_candidate: u32,
        next_unused_page: u32,
    },

    /// An empty table's free-space page has `FreeSpacePageHeader.next_page` != the sentinel.
    #[error(
        "Table {table_index} (empty): FreeSpacePageHeader.next_page should be sentinel \
         {NEXT_PAGE_SENTINEL:#010x}, but got {actual:?}"
    )]
    EmptyTableSentinelMissing {
        table_index: usize,
        actual: PageIndex,
    },

    /// An empty table's free-space page has `PageHeader.next_page` != `empty_candidate`.
    #[error(
        "Table {table_index} (empty): free-space page PageHeader.next_page should point to \
          empty_candidate {expected:?}, but got {actual:?}"
    )]
    EmptyTableIndexNextPageWrong {
        table_index: usize,
        expected: PageIndex,
        actual: PageIndex,
    },

    /// A non-empty table's last page has `PageHeader.next_page` != `empty_candidate`.
    #[error(
        "Table {table_index} (non-empty): last page PageHeader.next_page should point to \
         empty_candidate {expected:?}, but got {actual:?}"
    )]
    NonEmptyTableLastPageNextWrong {
        table_index: usize,
        expected: PageIndex,
        actual: PageIndex,
    },

    /// A non-empty table's free-space page has `FreeSpacePageHeader.next_page` != the first data page.
    #[error(
        "Table {table_index} (non-empty): FreeSpacePageHeader.next_page should be first data page \
         {expected:?}, but got {actual:?}"
    )]
    NonEmptyTableIndexNextPageWrong {
        table_index: usize,
        expected: PageIndex,
        actual: PageIndex,
    },

    /// A free-space page's `FreeSpacePageHeader.page_index` does not match `PageHeader.page_index`.
    #[error(
        "Free-space page {page_index:?}: FreeSpacePageHeader.page_index = {header_page_index:?} does not \
         match PageHeader.page_index"
    )]
    FreeSpaceHeaderPageIndexMismatch {
        page_index: PageIndex,
        header_page_index: PageIndex,
    },

    /// An IO or parse error occurred while loading a page during validation.
    #[error("IO/parse error while validating: {0}")]
    IoError(#[from] crate::util::RekordcrateError),
}

/// Validates all structural invariants of a PDB database.
///
/// Checks invariants derived from rekordbox-generated files:
/// - File size is a multiple of `page_size`
/// - `page_size == 4096`
/// - `next_page_sequence >= 1`
/// - `num_tables` matches the database type
/// - Every non-EC page has `PageHeader.page_index == physical position`
/// - Every non-EC page has `PageHeader.unknown2 == 0`
/// - Every non-EC page has valid `page_flags` (0x24, 0x34, or 0x64)
/// - Every free-space page has matching `PageHeader.page_index` and `FreeSpacePageHeader.page_index`
/// - Every table's `empty_candidate < next_unused_page`
/// - Empty tables have correct sentinel and pointer in their free-space page
/// - Non-empty tables have their last page pointing to `empty_candidate`
///
/// Returns a list of all violations found; an empty list means the database is valid.
pub fn validate<R: Read + Seek>(db: &mut Database<R>) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let db_type = db.db_type();

    check_page_size(db, &mut errors);
    check_sequence(db, &mut errors);
    check_num_tables(db, db_type, &mut errors);
    check_file_size(db, &mut errors);

    // Collect the set of EC page indices so we can skip them in per-page checks.
    // EC pages are allowed to have page_index == 0 (rekordbox style) or a valid
    // page_index (rekordcrate style), so we skip page_index / flags checks for them.
    let ec_pages: HashSet<u32> = db
        .get_header()
        .tables
        .iter()
        .map(|t| t.empty_candidate)
        .collect();

    let next_unused = db.get_header().next_unused_page.0;

    // Per-page checks.
    for physical_pos in 1..next_unused {
        let page_index = PageIndex(physical_pos);
        let is_ec = ec_pages.contains(&physical_pos);

        // EC pages may not yet exist in the file (rekordbox only writes them when
        // promoted to a data page). Skip all checks — they are implicitly valid.
        if is_ec {
            continue;
        }

        let page = match db.load_page(page_index) {
            Ok(p) => p,
            Err(e) => {
                errors.push(ValidationError::IoError(e));
                continue;
            }
        };

        if page.header.page_index.0 != physical_pos {
            errors.push(ValidationError::PageIndexMismatch {
                page_index: page.header.page_index,
                physical_position: physical_pos,
            });
        }

        if page.header.unknown2 != 0 {
            errors.push(ValidationError::PageUnknown2NonZero {
                page_index: page.header.page_index,
                value: page.header.unknown2,
            });
        }

        let flags_byte = page.header.page_flags.into_bytes()[0];
        if flags_byte != 0x24 && flags_byte != 0x34 && flags_byte != 0x64 {
            errors.push(ValidationError::InvalidPageFlags {
                page_index: page.header.page_index,
                flags: flags_byte,
            });
        }

        if page.header.page_flags.is_free_space_page() {
            if let Some(idx_content) = page.content.as_free_space() {
                if idx_content.header.page_index != page.header.page_index {
                    errors.push(ValidationError::FreeSpaceHeaderPageIndexMismatch {
                        page_index: page.header.page_index,
                        header_page_index: idx_content.header.page_index,
                    });
                }
            }
        }
    }

    // Per-table chain checks.
    let tables: Vec<Table> = db.get_header().tables.clone();
    for (table_index, table) in tables.iter().enumerate() {
        let ec = table.empty_candidate;
        let ec_idx = PageIndex(ec);
        let next_unused = db.get_header().next_unused_page.0;

        if ec >= next_unused {
            errors.push(ValidationError::EmptyCandidateOutOfBounds {
                table_index,
                empty_candidate: ec,
                next_unused_page: next_unused,
            });
            continue;
        }

        if table.first_page == table.last_page {
            // Empty table: the free-space page is both first and last.
            let free_space_page = match db.load_page(table.first_page) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(ValidationError::IoError(e));
                    continue;
                }
            };

            if free_space_page.header.next_page != ec_idx {
                errors.push(ValidationError::EmptyTableIndexNextPageWrong {
                    table_index,
                    expected: ec_idx,
                    actual: free_space_page.header.next_page,
                });
            }

            if let Some(idx_content) = free_space_page.content.as_free_space() {
                if idx_content.header.next_page.0 != NEXT_PAGE_SENTINEL {
                    errors.push(ValidationError::EmptyTableSentinelMissing {
                        table_index,
                        actual: idx_content.header.next_page,
                    });
                }
            }
        } else {
            // Non-empty table: last page must link to the EC page.
            let last_page = match db.load_page(table.last_page) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(ValidationError::IoError(e));
                    continue;
                }
            };

            if last_page.header.next_page != ec_idx {
                errors.push(ValidationError::NonEmptyTableLastPageNextWrong {
                    table_index,
                    expected: ec_idx,
                    actual: last_page.header.next_page,
                });
            }

            // The free-space page's inner FreeSpacePageHeader.next_page must agree with the outer
            // PageHeader.next_page (both should point to the first data page).
            let free_space_page = match db.load_page(table.first_page) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(ValidationError::IoError(e));
                    continue;
                }
            };
            let expected_first_data = free_space_page.header.next_page;
            if let Some(idx_content) = free_space_page.content.as_free_space() {
                if idx_content.header.next_page != expected_first_data {
                    errors.push(ValidationError::NonEmptyTableIndexNextPageWrong {
                        table_index,
                        expected: expected_first_data,
                        actual: idx_content.header.next_page,
                    });
                }
            }
        }
    }

    errors
}

fn check_page_size<R: Read + Seek>(db: &Database<R>, errors: &mut Vec<ValidationError>) {
    let page_size = db.get_header().page_size;
    if page_size != EXPECTED_PAGE_SIZE {
        errors.push(ValidationError::UnexpectedPageSize {
            expected: EXPECTED_PAGE_SIZE,
            actual: page_size,
        });
    }
}

fn check_sequence<R: Read + Seek>(db: &Database<R>, errors: &mut Vec<ValidationError>) {
    let seq = db.get_header().next_page_sequence;
    if seq == 0 {
        errors.push(ValidationError::SequenceZero(seq));
    }
}

fn check_num_tables<R: Read + Seek>(
    db: &Database<R>,
    db_type: DatabaseType,
    errors: &mut Vec<ValidationError>,
) {
    let expected = match db_type {
        DatabaseType::Plain => EXPECTED_NUM_TABLES_PLAIN,
        DatabaseType::Ext => EXPECTED_NUM_TABLES_EXT,
    };
    let actual = db.get_header().num_tables;
    if actual != expected {
        errors.push(ValidationError::WrongNumTables { expected, actual });
    }
}

fn check_file_size<R: Read + Seek>(db: &mut Database<R>, errors: &mut Vec<ValidationError>) {
    let page_size = db.get_header().page_size;
    match db.file_size() {
        Ok(size) if size % u64::from(page_size) != 0 => {
            errors.push(ValidationError::FileSizeNotMultipleOfPageSize {
                file_size: size,
                page_size,
            });
        }
        _ => {}
    }
}
