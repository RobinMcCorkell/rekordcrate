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
use rekordcrate::pdb::validation::validate;
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
    assert_eq!(header.unknown, 5);
    assert_eq!(header.next_page_sequence, 1);
}

#[test]
fn test_insert_initial_content_updates_sequence_like_empty_export() {
    let cursor = Cursor::new(Vec::new());
    let mut db = Database::create(cursor, DatabaseType::Plain).expect("failed to create database");
    insert_initial_content(&mut db, "2025-10-29".parse().unwrap(), "".parse().unwrap())
        .expect("insert_initial_content failed");

    assert_eq!(db.get_header().next_page_sequence, 6);
}

/// Verifies that each table has an free-space page as both first_page and last_page (empty table),
/// with an empty_candidate pointing at the pre-allocated data page.
/// Index pages are odd-numbered, data pages even-numbered (rekordbox convention).
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
            "table {}: first_page should be free-space page {:?}",
            i, expected_index_page
        );
        // For an empty table, last_page == first_page (free-space page only).
        assert_eq!(
            table.last_page, expected_index_page,
            "table {}: last_page should be free-space page {:?} for an empty table",
            i, expected_index_page
        );
        // The pre-allocated empty_candidate is the data page immediately after the free-space page.
        assert_eq!(
            PageIndex::try_from(table.empty_candidate).unwrap(),
            expected_data_page,
            "table {}: empty_candidate should be data page {:?}",
            i,
            expected_data_page
        );
    }
}

/// Verifies that iterating an empty table's pages yields exactly one free-space page (no data page in
/// the chain — the pre-allocated data page is outside the chain until the first row is inserted).
#[test]
fn test_create_page_chain_has_free_space_only() {
    let mut db = create_empty_db();
    let tables = db.get_header().tables.clone();

    for (i, table) in tables.iter().enumerate() {
        let table_idx = TableIndex::from(i);
        let mut page_iter = db
            .iter_pages_for_table(table_idx)
            .expect("failed to get page iterator");

        // First page must be the free-space page.
        let index_page = page_iter
            .next()
            .expect("page iterator error")
            .expect("expected an free-space page but got None");

        assert_eq!(
            index_page.header.page_index, table.first_page,
            "table {}: first page index mismatch",
            i
        );
        assert!(
            index_page.header.page_flags.is_free_space_page(),
            "table {}: page {:?} should have is_free_space_page=true",
            i,
            index_page.header.page_index
        );

        // For an empty table, the chain must stop after the free-space page (last_page == first_page).
        assert!(
            page_iter.next().expect("page iterator error").is_none(),
            "table {}: expected exactly 1 page (index only) in chain, but found more",
            i
        );
    }
}

/// Verifies the properties of free-space pages that CDJ hardware requires: is_free_space_page flag,
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
            .expect("expected an free-space page");

        assert!(
            index_page.header.page_flags.is_free_space_page(),
            "table {}: free-space page must have is_free_space_page flag set",
            i
        );
        assert_eq!(
            index_page.header.free_size, 0,
            "table {}: free-space page free_size must be 0 (no row data)",
            i
        );
        assert_eq!(
            index_page.header.used_size, 0,
            "table {}: free-space page used_size must be 0",
            i
        );
        assert_eq!(
            index_page.header.page_sequence, 1,
            "table {}: free-space page sequence must be 1 (matches Rekordbox)",
            i
        );
        // PageHeader.next_page on the free-space page must point to the pre-allocated ec page.
        assert_eq!(
            index_page.header.next_page,
            PageIndex::try_from(table.empty_candidate).unwrap(),
            "table {}: free-space page next_page must point to ec page",
            i
        );

        // Verify FreeSpacePageContent has the expected sentinel values for an empty table.
        let index_content = index_page
            .content
            .as_free_space()
            .expect("free-space page must have Index content");

        assert_eq!(
            index_content.header.unknown_a, 0x1FFF,
            "table {}: free-space content unknown_a must be 0x1FFF",
            i
        );
        assert_eq!(
            index_content.header.unknown_b, 0x1FFF,
            "table {}: free-space content unknown_b must be 0x1FFF",
            i
        );
        assert_eq!(
            index_content.header.next_offset, 0,
            "table {}: free-space content next_offset must be 0",
            i
        );
        assert_eq!(
            index_content.header.num_entries, 0,
            "table {}: free-space content num_entries must be 0",
            i
        );
        assert_eq!(
            index_content.header.first_empty, 0x1FFF,
            "table {}: free-space content first_empty must be 0x1FFF (empty sentinel)",
            i
        );
        assert_eq!(
            index_content.header.page_index, table.first_page,
            "table {}: free-space content page_index must match table first_page",
            i
        );
        // FreeSpacePageHeader.next_page must be the null sentinel (0x03FFFFFF) for an empty table.
        assert_eq!(
            index_content.header.next_page,
            PageIndex::try_from(0x03FF_FFFFu32).unwrap(),
            "table {}: free-space content next_page must be 0x03FFFFFF null sentinel",
            i
        );
        assert!(
            index_content.entries.is_empty(),
            "table {}: free-space page must have no entries",
            i
        );
    }
}

/// Verifies that the pre-allocated empty-candidate (ec) data page exists for each table and has
/// the correct properties: not an free-space page, full free space, zero used space, matching type.
#[test]
fn test_create_data_page_properties() {
    let mut db = create_empty_db();
    let tables = db.get_header().tables.clone();

    for (i, table) in tables.iter().enumerate() {
        let ec_page_idx = PageIndex::try_from(table.empty_candidate)
            .expect("empty_candidate must be a valid page index");

        let ec_page = db.load_page(ec_page_idx).expect("failed to load ec page");

        assert!(
            !ec_page.header.page_flags.is_free_space_page(),
            "table {}: ec page must not have is_free_space_page flag",
            i
        );
        assert_eq!(
            ec_page.header.free_size, DATA_PAGE_FREE_SIZE,
            "table {}: empty ec page must have full free space ({} bytes)",
            i, DATA_PAGE_FREE_SIZE
        );
        assert_eq!(
            ec_page.header.used_size, 0,
            "table {}: empty ec page must have used_size = 0",
            i
        );
        assert_eq!(
            ec_page.header.page_type, table.page_type,
            "table {}: ec page type must match table page_type",
            i
        );
        assert_eq!(
            ec_page.header.page_index, ec_page_idx,
            "table {}: ec page index must match empty_candidate",
            i
        );
        assert!(
            ec_page.content.as_data().is_some(),
            "table {}: ec page must have Data content",
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

/// Verifies that after inserting default content, rows land in data pages (not free-space pages),
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

        // The first page in the chain must still be the free-space page.
        let first_page = page_iter
            .next()
            .expect("page iterator error")
            .expect("expected first page");
        assert!(
            first_page.header.page_flags.is_free_space_page(),
            "table {}: first page must remain an free-space page after inserts",
            i
        );

        // All subsequent pages must be data pages.
        while let Some(page) = page_iter.next().expect("page iterator error") {
            assert!(
                !page.header.page_flags.is_free_space_page(),
                "table {}: page {:?} must be a data page (not free-space page) after inserts",
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

/// Verifies that a freshly created empty database passes all structural validation checks.
#[test]
fn test_validate_empty_database() {
    let mut db = create_empty_db();
    let errors = validate(&mut db);
    assert!(
        errors.is_empty(),
        "empty database has validation errors: {errors:#?}"
    );
}

/// Verifies that the file bytes of an empty database are an exact multiple of page_size.
#[test]
fn test_empty_database_file_size_is_multiple_of_page_size() {
    let mut data = Vec::new();
    {
        let cursor = Cursor::new(&mut data);
        Database::create(cursor, DatabaseType::Plain)
            .expect("failed to create database")
            .close()
            .expect("failed to close");
    }
    assert_eq!(
        data.len() % PAGE_SIZE as usize,
        0,
        "empty database file size {} is not a multiple of page_size {}",
        data.len(),
        PAGE_SIZE
    );
    // Unwritten data pages still leave sparse gaps up to the last written free-space page (39).
    let expected_size = PAGE_SIZE as usize * (NUM_TABLES * 2);
    assert_eq!(
        data.len(),
        expected_size,
        "expected file size {expected_size}, got {}",
        data.len()
    );
}

/// Verifies that a database with default content passes all structural validation checks.
#[test]
fn test_validate_database_with_defaults() {
    let mut data = Vec::new();
    {
        let cursor = Cursor::new(&mut data);
        let mut db =
            Database::create(cursor, DatabaseType::Plain).expect("failed to create database");
        insert_initial_content(&mut db, "2025-10-29".parse().unwrap(), "".parse().unwrap())
            .expect("insert_initial_content failed");
        insert_standard_keys(&mut db).expect("insert_standard_keys failed");
        db.close().expect("failed to close");
    }

    let cursor = Cursor::new(data.as_slice());
    let mut db = Database::open_non_persistent(cursor, DatabaseType::Plain)
        .expect("failed to re-open database");
    let errors = validate(&mut db);
    assert!(
        errors.is_empty(),
        "database with default content has validation errors: {errors:#?}"
    );
}

/// Verifies that a database with default content has a file size that is a multiple of page_size.
#[test]
fn test_database_with_defaults_file_size_is_multiple_of_page_size() {
    let mut data = Vec::new();
    {
        let cursor = Cursor::new(&mut data);
        let mut db =
            Database::create(cursor, DatabaseType::Plain).expect("failed to create database");
        insert_initial_content(&mut db, "2025-10-29".parse().unwrap(), "".parse().unwrap())
            .expect("insert_initial_content failed");
        insert_standard_keys(&mut db).expect("insert_standard_keys failed");
        db.close().expect("failed to close");
    }
    assert_eq!(
        data.len() % PAGE_SIZE as usize,
        0,
        "database file size {} is not a multiple of page_size {}",
        data.len(),
        PAGE_SIZE
    );
}
