// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Tests for [`Database::create()`], verifying structural invariants required for CDJ hardware
//! compatibility.

use fallible_iterator::FallibleIterator;
use rekordcrate::pdb::defaults::{insert_initial_content, insert_standard_keys};
use rekordcrate::pdb::io::Database;
use rekordcrate::pdb::*;
use rekordcrate::util::TableIndex;
use std::io::Cursor;

const PAGE_SIZE: u32 = 4096;
const NUM_TABLES: usize = 20;
/// Free space in an empty data page: PAGE_SIZE minus the page header (0x20 = 32 bytes) and the
/// data page header (0x8 = 8 bytes).
const DATA_PAGE_FREE_SIZE: u16 = (PAGE_SIZE - PageHeader::BINARY_SIZE - 8) as u16;

fn create_empty_db() -> Database<Cursor<Vec<u8>>> {
    let cursor = Cursor::new(Vec::new());
    Database::create(cursor, DatabaseType::Plain).expect("failed to create database")
}

fn get_row_count<RowT: RowVariant>(db: &mut Database<impl std::io::Read + std::io::Seek>) -> usize {
    db.iter_rows::<RowT>()
        .expect("failed to get row iterator")
        .count()
        .expect("failed to count rows")
}

/// Verifies header fields that a CDJ reads to understand the database structure.
#[test]
fn test_create_header_fields() {
    let db = create_empty_db();
    let header = db.get_header();

    assert_eq!(header.page_size, PAGE_SIZE);
    assert_eq!(header.num_tables, NUM_TABLES as u32);
    // Each table gets 2 pages (index + data), so next_unused = 2*NUM_TABLES + 1.
    assert_eq!(
        header.next_unused_page,
        PageIndex::try_from(NUM_TABLES as u32 * 2 + 1).unwrap()
    );
    // unknown=1 matches Rekordbox format; hardware may use this as an "initialized" flag.
    assert_eq!(header.unknown, 1);
    assert_eq!(header.sequence, 1);
}

/// Verifies that each table has an index page as first_page and a data page as last_page,
/// following Rekordbox's page numbering scheme: index pages are odd, data pages are even.
#[test]
fn test_create_two_pages_per_table() {
    let db = create_empty_db();
    let header = db.get_header();

    assert_eq!(header.tables.len(), NUM_TABLES);

    for (i, table) in header.tables.iter().enumerate() {
        let expected_index_page = PageIndex::try_from(i as u32 * 2 + 1).unwrap(); // 1, 3, 5, ..., 39
        let expected_data_page = PageIndex::try_from(i as u32 * 2 + 2).unwrap(); //  2, 4, 6, ..., 40

        assert_eq!(
            table.first_page, expected_index_page,
            "table {}: first_page should be index page {:?}",
            i,
            expected_index_page
        );
        assert_eq!(
            table.last_page, expected_data_page,
            "table {}: last_page should be data page {:?}",
            i,
            expected_data_page
        );
    }
}

/// Verifies that iterating a table's pages yields exactly one index page followed by one data page.
#[test]
fn test_create_page_chain_has_index_then_data() {
    let mut db = create_empty_db();
    let tables = db.get_header().tables.clone();

    for (i, table) in tables.iter().enumerate() {
        let table_idx = TableIndex::from(i);
        let mut page_iter = db
            .iter_pages_for_table(table_idx)
            .expect("failed to get page iterator");

        // First page must be the index page.
        let index_page = page_iter
            .next()
            .expect("page iterator error")
            .expect("expected an index page but got None");

        assert_eq!(
            index_page.header.page_index, table.first_page,
            "table {}: first page index mismatch",
            i
        );
        assert!(
            index_page.header.page_flags.is_index_page(),
            "table {}: page {:?} should have is_index_page=true",
            i,
            index_page.header.page_index
        );

        // Second page must be the data page.
        let data_page = page_iter
            .next()
            .expect("page iterator error")
            .expect("expected a data page but got None");

        assert_eq!(
            data_page.header.page_index, table.last_page,
            "table {}: last page index mismatch",
            i
        );
        assert!(
            !data_page.header.page_flags.is_index_page(),
            "table {}: page {:?} should have is_index_page=false",
            i,
            data_page.header.page_index
        );

        // The chain must stop after exactly 2 pages.
        assert!(
            page_iter
                .next()
                .expect("page iterator error")
                .is_none(),
            "table {}: expected exactly 2 pages in chain, but found more",
            i
        );
    }
}

/// Verifies the properties of index pages that CDJ hardware requires: is_index_page flag,
/// zero free/used space, and correct links into the data page.
#[test]
fn test_create_index_page_properties() {
    let mut db = create_empty_db();
    let tables = db.get_header().tables.clone();

    for (i, table) in tables.iter().enumerate() {
        let table_idx = TableIndex::from(i);
        let mut page_iter = db
            .iter_pages_for_table(table_idx)
            .expect("failed to get page iterator");

        let index_page = page_iter
            .next()
            .expect("page iterator error")
            .expect("expected an index page");

        assert!(
            index_page.header.page_flags.is_index_page(),
            "table {}: index page must have is_index_page flag set",
            i
        );
        assert_eq!(
            index_page.header.free_size, 0,
            "table {}: index page free_size must be 0 (no row data)",
            i
        );
        assert_eq!(
            index_page.header.used_size, 0,
            "table {}: index page used_size must be 0",
            i
        );
        assert_eq!(
            index_page.header.unknown1, 1,
            "table {}: index page unknown1 must be 1 (matches Rekordbox)",
            i
        );
        // Index page's next_page must link to the data page.
        assert_eq!(
            index_page.header.next_page, table.last_page,
            "table {}: index page next_page must point to data page",
            i
        );

        // Verify IndexPageContent has the expected sentinel values for an empty index.
        let index_content = index_page
            .content
            .as_index()
            .expect("index page must have Index content");

        assert_eq!(
            index_content.header.unknown_a, 0x1FFF,
            "table {}: index content unknown_a must be 0x1FFF",
            i
        );
        assert_eq!(
            index_content.header.unknown_b, 0x1FFF,
            "table {}: index content unknown_b must be 0x1FFF",
            i
        );
        assert_eq!(
            index_content.header.next_offset, 0,
            "table {}: index content next_offset must be 0",
            i
        );
        assert_eq!(
            index_content.header.num_entries, 0,
            "table {}: index content num_entries must be 0",
            i
        );
        assert_eq!(
            index_content.header.first_empty, 0x1FFF,
            "table {}: index content first_empty must be 0x1FFF (empty sentinel)",
            i
        );
        assert_eq!(
            index_content.header.page_index, table.first_page,
            "table {}: index content page_index must match table first_page",
            i
        );
        assert!(
            index_content.entries.is_empty(),
            "table {}: index page must have no entries",
            i
        );
    }
}

/// Verifies data page properties: no is_index_page flag, full free space, matching page type.
#[test]
fn test_create_data_page_properties() {
    let mut db = create_empty_db();
    let tables = db.get_header().tables.clone();

    for (i, table) in tables.iter().enumerate() {
        let table_idx = TableIndex::from(i);
        let mut page_iter = db
            .iter_pages_for_table(table_idx)
            .expect("failed to get page iterator");

        // Skip the index page.
        page_iter.next().expect("page iterator error");

        let data_page = page_iter
            .next()
            .expect("page iterator error")
            .expect("expected a data page");

        assert!(
            !data_page.header.page_flags.is_index_page(),
            "table {}: data page must not have is_index_page flag",
            i
        );
        assert_eq!(
            data_page.header.free_size, DATA_PAGE_FREE_SIZE,
            "table {}: empty data page must have full free space ({} bytes)",
            i,
            DATA_PAGE_FREE_SIZE
        );
        assert_eq!(
            data_page.header.used_size, 0,
            "table {}: empty data page must have used_size = 0",
            i
        );
        assert_eq!(
            data_page.header.page_type, table.page_type,
            "table {}: data page type must match table page_type",
            i
        );
        assert_eq!(
            data_page.header.page_index, table.last_page,
            "table {}: data page index must match table last_page",
            i
        );
        // Data page content must be an empty Data variant.
        assert!(
            data_page.content.as_data().is_some(),
            "table {}: data page must have Data content",
            i
        );
    }
}

/// Verifies that all tables are empty immediately after creation.
#[test]
fn test_create_all_tables_empty() {
    let mut db = create_empty_db();

    assert_eq!(get_row_count::<Album>(&mut db), 0);
    assert_eq!(get_row_count::<Artist>(&mut db), 0);
    assert_eq!(get_row_count::<Artwork>(&mut db), 0);
    assert_eq!(get_row_count::<Color>(&mut db), 0);
    assert_eq!(get_row_count::<ColumnEntry>(&mut db), 0);
    assert_eq!(get_row_count::<Genre>(&mut db), 0);
    assert_eq!(get_row_count::<HistoryEntry>(&mut db), 0);
    assert_eq!(get_row_count::<HistoryPlaylist>(&mut db), 0);
    assert_eq!(get_row_count::<History>(&mut db), 0);
    assert_eq!(get_row_count::<Key>(&mut db), 0);
    assert_eq!(get_row_count::<Label>(&mut db), 0);
    assert_eq!(get_row_count::<Menu>(&mut db), 0);
    assert_eq!(get_row_count::<PlaylistEntry>(&mut db), 0);
    assert_eq!(get_row_count::<PlaylistTreeNode>(&mut db), 0);
    assert_eq!(get_row_count::<Track>(&mut db), 0);
}

/// Verifies that after inserting default content, rows land in data pages (not index pages),
/// and the page chain structure is preserved.
#[test]
fn test_create_inserts_go_to_data_pages() {
    let cursor = Cursor::new(Vec::new());
    let mut db = Database::create(cursor, DatabaseType::Plain).expect("failed to create database");
    insert_initial_content(&mut db, "2025-10-29".parse().unwrap(), "".parse().unwrap())
        .expect("insert_initial_content failed");
    insert_standard_keys(&mut db).expect("insert_standard_keys failed");

    let tables = db.get_header().tables.clone();

    for (i, table) in tables.iter().enumerate() {
        if matches!(table.page_type, PageType::Unknown(_)) {
            continue;
        }

        let table_idx = TableIndex::from(i);
        let mut page_iter = db
            .iter_pages_for_table(table_idx)
            .expect("failed to get page iterator");

        // The first page in the chain must still be the index page.
        let first_page = page_iter
            .next()
            .expect("page iterator error")
            .expect("expected first page");
        assert!(
            first_page.header.page_flags.is_index_page(),
            "table {}: first page must remain an index page after inserts",
            i
        );

        // All subsequent pages must be data pages.
        while let Some(page) = page_iter.next().expect("page iterator error") {
            assert!(
                !page.header.page_flags.is_index_page(),
                "table {}: page {:?} must be a data page (not index page) after inserts",
                i,
                page.header.page_index
            );
            assert!(
                page.content.as_data().is_some(),
                "table {}: page {:?} must have Data content",
                i,
                page.header.page_index
            );
        }
    }
}

/// Verifies that `insert_initial_content` and `insert_standard_keys` produce the expected row
/// counts, and that the database can be serialized and re-opened with the data intact.
#[test]
fn test_create_roundtrip_with_defaults() {
    let mut data = Vec::new();
    {
        let cursor = Cursor::new(&mut data);
        let mut db =
            Database::create(cursor, DatabaseType::Plain).expect("failed to create database");
        insert_initial_content(&mut db, "2025-10-29".parse().unwrap(), "".parse().unwrap())
            .expect("insert_initial_content failed");
        insert_standard_keys(&mut db).expect("insert_standard_keys failed");
        db.close().expect("failed to close database");
    }

    // Re-open and verify row counts are preserved after serialization.
    let cursor = Cursor::new(data.as_slice());
    let mut db = Database::open_non_persistent(cursor, DatabaseType::Plain)
        .expect("failed to re-open database");

    assert_eq!(get_row_count::<Color>(&mut db), 8);
    assert_eq!(get_row_count::<ColumnEntry>(&mut db), 27);
    assert_eq!(get_row_count::<Menu>(&mut db), 22);
    assert_eq!(get_row_count::<History>(&mut db), 1);
    assert_eq!(get_row_count::<Key>(&mut db), 24);
}
