// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use rekordcrate::pdb::io::Database;
use rekordcrate::pdb::{DatabaseType, PageType, PlainPageType};
use std::io::Cursor;

fn assert_pdb_row_count(page_type: PlainPageType, expected_row_count: usize) {
    let data = include_bytes!("../data/pdb/num_rows/export.pdb").as_slice();
    let mut reader = Cursor::new(data);
    let mut db = Database::open_non_persistent(&mut reader, DatabaseType::Plain)
        .expect("Failed to open database");

    let page_iter = db
        .load_pages(PageType::Plain(page_type))
        .expect("Failed to load pages for page type");

    let actual_row_count: usize = page_iter
        .filter_map(|page| {
            page.content
                .as_data()
                .map(|data_content| data_content.rows.len())
        })
        .try_fold(0, |acc, res| res.map(|len| acc + len))
        .expect("Failed to count rows");

    assert_eq!(
        actual_row_count, expected_row_count,
        "wrong row count for page type {:?}",
        page_type
    );
}

#[test]
fn test_pdb_row_count_albums() {
    assert_pdb_row_count(PlainPageType::Albums, 2226);
}

#[test]
fn test_pdb_row_count_artists() {
    assert_pdb_row_count(PlainPageType::Artists, 2216);
}

#[test]
fn test_pdb_row_count_artwork() {
    assert_pdb_row_count(PlainPageType::Artwork, 2178);
}

#[test]
fn test_pdb_row_count_colors() {
    assert_pdb_row_count(PlainPageType::Colors, 8);
}

#[test]
fn test_pdb_row_count_genres() {
    assert_pdb_row_count(PlainPageType::Genres, 315);
}

#[test]
fn test_pdb_row_count_historyplaylists() {
    assert_pdb_row_count(PlainPageType::HistoryPlaylists, 1);
}

#[test]
fn test_pdb_row_count_historyentries() {
    assert_pdb_row_count(PlainPageType::HistoryEntries, 73);
}

#[test]
fn test_pdb_row_count_keys() {
    assert_pdb_row_count(PlainPageType::Keys, 67);
}

#[test]
fn test_pdb_row_count_labels() {
    assert_pdb_row_count(PlainPageType::Labels, 688);
}

#[test]
fn test_pdb_row_count_playlisttree() {
    assert_pdb_row_count(PlainPageType::PlaylistTree, 104);
}

#[test]
fn test_pdb_row_count_playlistentries() {
    assert_pdb_row_count(PlainPageType::PlaylistEntries, 7440);
}

#[test]
fn test_pdb_row_count_columns() {
    assert_pdb_row_count(PlainPageType::Columns, 27);
}

#[test]
fn test_pdb_row_count_tracks() {
    assert_pdb_row_count(PlainPageType::Tracks, 3886);
}
