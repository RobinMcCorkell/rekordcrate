// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Default seed rows for Pioneer DeviceSQL databases.

use std::io::{Read, Seek, Write};

use super::io::Database;
use super::string::DeviceSQLString;
use super::{
    Color, ColumnEntry, History, Key, KeyId, Menu, MenuVisibility, PlainRow, Row, Unknown18Row,
};
use crate::util::ColorIndex;
use crate::Result;

/// Rekordbox's history row version string for exported PDB files.
pub const DEFAULT_HISTORY_VERSION: &str = "1000";

/// Insert the rows present in a freshly exported blank `export.pdb`.
///
/// This mirrors `data/incremental/000/export.pdb`: colors, metadata columns,
/// menu definitions, table-18 seed rows, and a single history sync row. It
/// intentionally does not insert any key rows because Rekordbox leaves that
/// table empty in a blank export.
pub fn insert_initial_content<IO: Read + Write + Seek>(
    db: &mut Database<IO>,
    history_date: DeviceSQLString,
    history_label: DeviceSQLString,
) -> Result<()> {
    insert_default_history_row(db, history_date, history_label)?;
    insert_standard_colors(db)?;
    insert_default_columns(db)?;
    insert_default_menu_rows(db)?;
    insert_default_unknown18_rows(db)?;
    Ok(())
}

/// Insert the 24 canonical Rekordbox key rows used by populated exports.
pub fn insert_standard_keys<IO: Read + Write + Seek>(db: &mut Database<IO>) -> Result<()> {
    for &(id, name) in STANDARD_KEYS {
        db.insert_row(Row::Plain(PlainRow::Key(Key::new(
            KeyId(id),
            name.parse()?,
        ))))?;
    }
    Ok(())
}

/// Insert the standard Rekordbox color palette.
pub fn insert_standard_colors<IO: Read + Write + Seek>(db: &mut Database<IO>) -> Result<()> {
    for (i, &(ref color, name)) in STANDARD_COLORS.iter().enumerate() {
        db.insert_row(Row::Plain(PlainRow::Color(Color::new(
            color.clone(),
            (i + 1) as u8,
            name.parse()?,
        ))))?;
    }
    Ok(())
}

/// Insert the default metadata browsing columns.
pub fn insert_default_columns<IO: Read + Write + Seek>(db: &mut Database<IO>) -> Result<()> {
    for &(id, kind, name) in DEFAULT_COLUMNS {
        db.insert_row(Row::Plain(PlainRow::ColumnEntry(ColumnEntry::new(
            id,
            kind,
            name.parse()?,
        ))))?;
    }
    Ok(())
}

/// Insert the default CDJ browse menu rows.
pub fn insert_default_menu_rows<IO: Read + Write + Seek>(db: &mut Database<IO>) -> Result<()> {
    for &(category_id, content_pointer, unknown, visibility, sort_order) in DEFAULT_MENU_ROWS {
        db.insert_row(Row::Plain(PlainRow::Menu(Menu::new(
            category_id,
            content_pointer,
            unknown,
            visibility,
            sort_order,
        ))))?;
    }
    Ok(())
}

/// Insert the initial history sync row.
pub fn insert_default_history_row<IO: Read + Write + Seek>(
    db: &mut Database<IO>,
    history_date: DeviceSQLString,
    history_label: DeviceSQLString,
) -> Result<()> {
    db.insert_row(Row::Plain(PlainRow::History(History::new(
        0,
        history_date,
        DEFAULT_HISTORY_VERSION.parse()?,
        history_label,
    ))))?;
    Ok(())
}

/// Insert the fixed rows present in table 18 of a blank Rekordbox export.
pub fn insert_default_unknown18_rows<IO: Read + Write + Seek>(db: &mut Database<IO>) -> Result<()> {
    for row in DEFAULT_UNKNOWN18_ROWS {
        db.insert_row(Row::Unknown18(*row))?;
    }

    Ok(())
}

const STANDARD_KEYS: &[(u32, &str)] = &[
    (1, "Dm"),
    (2, "Abm"),
    (3, "Cm"),
    (4, "Bbm"),
    (5, "Fm"),
    (6, "Dbm"),
    (7, "Bm"),
    (8, "Gm"),
    (9, "F#m"),
    (10, "Em"),
    (11, "Am"),
    (12, "D"),
    (13, "Ebm"),
    (14, "Bb"),
    (15, "G"),
    (16, "Db"),
    (17, "A"),
    (18, "B"),
    (19, "C"),
    (20, "F"),
    (21, "Ab"),
    (22, "Eb"),
    (23, "F#"),
    (24, "E"),
];

const STANDARD_COLORS: &[(ColorIndex, &str)] = &[
    (ColorIndex::Pink, "Pink"),
    (ColorIndex::Red, "Red"),
    (ColorIndex::Orange, "Orange"),
    (ColorIndex::Yellow, "Yellow"),
    (ColorIndex::Green, "Green"),
    (ColorIndex::Aqua, "Aqua"),
    (ColorIndex::Blue, "Blue"),
    (ColorIndex::Purple, "Purple"),
];

const DEFAULT_COLUMNS: &[(u16, u16, &str)] = &[
    (1, 128, "￺GENRE￻"),
    (2, 129, "￺ARTIST￻"),
    (3, 130, "￺ALBUM￻"),
    (4, 131, "￺TRACK￻"),
    (5, 133, "￺BPM￻"),
    (6, 134, "￺RATING￻"),
    (7, 135, "￺YEAR￻"),
    (8, 136, "￺REMIXER￻"),
    (9, 137, "￺LABEL￻"),
    (10, 138, "￺ORIGINAL ARTIST￻"),
    (11, 139, "￺KEY￻"),
    (12, 141, "￺CUE￻"),
    (13, 142, "￺COLOR￻"),
    (14, 146, "￺TIME￻"),
    (15, 147, "￺BITRATE￻"),
    (16, 148, "￺FILE NAME￻"),
    (17, 132, "￺PLAYLIST￻"),
    (18, 152, "￺HOT CUE BANK￻"),
    (19, 149, "￺HISTORY￻"),
    (20, 145, "￺SEARCH￻"),
    (21, 150, "￺COMMENTS￻"),
    (22, 140, "￺DATE ADDED￻"),
    (23, 151, "￺DJ PLAY COUNT￻"),
    (24, 144, "￺FOLDER￻"),
    (25, 161, "￺DEFAULT￻"),
    (26, 162, "￺ALPHABET￻"),
    (27, 170, "￺MATCHING￻"),
];

const DEFAULT_MENU_ROWS: &[(u16, u16, u8, MenuVisibility, u16)] = &[
    (1, 1, 99, MenuVisibility::Hidden, 0),
    (5, 6, 5, MenuVisibility::Hidden, 0),
    (6, 7, 99, MenuVisibility::Hidden, 0),
    (7, 8, 99, MenuVisibility::Hidden, 0),
    (8, 9, 99, MenuVisibility::Hidden, 0),
    (9, 10, 99, MenuVisibility::Hidden, 0),
    (10, 11, 99, MenuVisibility::Hidden, 0),
    (13, 15, 99, MenuVisibility::Hidden, 0),
    (14, 19, 4, MenuVisibility::Hidden, 0),
    (15, 20, 6, MenuVisibility::Hidden, 0),
    (16, 21, 99, MenuVisibility::Hidden, 0),
    (18, 23, 99, MenuVisibility::Hidden, 0),
    (2, 2, 2, MenuVisibility::Visible, 1),
    (3, 3, 3, MenuVisibility::Visible, 2),
    (4, 4, 1, MenuVisibility::Visible, 3),
    (11, 12, 99, MenuVisibility::Visible, 4),
    (17, 5, 99, MenuVisibility::Visible, 5),
    (19, 22, 99, MenuVisibility::Visible, 6),
    (20, 18, 99, MenuVisibility::Visible, 7),
    (27, 26, 99, MenuVisibility::Unknown(2), 8),
    (24, 17, 99, MenuVisibility::Visible, 9),
    (22, 27, 99, MenuVisibility::Visible, 10),
];

const DEFAULT_UNKNOWN18_ROWS: &[Unknown18Row] = &[
    Unknown18Row::new(1, 6, 0x0001, 0),
    Unknown18Row::new(21, 7, 0x0001, 0),
    Unknown18Row::new(14, 8, 0x0001, 0),
    Unknown18Row::new(8, 9, 0x0001, 0),
    Unknown18Row::new(9, 10, 0x0001, 0),
    Unknown18Row::new(10, 11, 0x0001, 0),
    Unknown18Row::new(15, 13, 0x0001, 0),
    Unknown18Row::new(13, 15, 0x0001, 0),
    Unknown18Row::new(23, 16, 0x0001, 0),
    Unknown18Row::new(22, 17, 0x0001, 0),
    Unknown18Row::new(25, 0, 0x0100, 0),
    Unknown18Row::new(26, 1, 0x0200, 0),
    Unknown18Row::new(2, 2, 0x0300, 0),
    Unknown18Row::new(3, 3, 0x0400, 0),
    Unknown18Row::new(5, 4, 0x0500, 0),
    Unknown18Row::new(6, 5, 0x0600, 0),
    Unknown18Row::new(11, 12, 0x0700, 0),
];

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::pdb::{DatabaseType, PageContent, PageIndex, PageType};
    use fallible_iterator::FallibleIterator;

    #[test]
    fn blank_export_defaults_match_expected_counts() -> Result<()> {
        let cursor = Cursor::new(Vec::new());
        let mut db = Database::create(cursor, DatabaseType::Plain)?;

        insert_initial_content(&mut db, "2025-10-29".parse()?, "".parse()?)?;

        assert_eq!(db.iter_rows::<Color>()?.count()?, 8);
        assert_eq!(db.iter_rows::<ColumnEntry>()?.count()?, 27);
        assert_eq!(db.iter_rows::<Menu>()?.count()?, 22);
        assert_eq!(db.iter_rows::<History>()?.count()?, 1);
        assert_eq!(db.iter_rows::<Key>()?.count()?, 0);
        assert_eq!(db.iter_rows::<Unknown18Row>()?.count()?, 17);

        insert_standard_keys(&mut db)?;
        assert_eq!(db.iter_rows::<Key>()?.count()?, 24);

        Ok(())
    }

    #[test]
    fn blank_export_defaults_seed_table18_like_rekordbox() -> Result<()> {
        let cursor = Cursor::new(Vec::new());
        let mut db = Database::create(cursor, DatabaseType::Plain)?;

        insert_initial_content(&mut db, "2025-10-29".parse()?, "".parse()?)?;

        let table18 = db
            .get_header()
            .find_table(PageType::Unknown(18))
            .expect("plain PDBs always contain table 18")
            .1
            .clone();

        assert_eq!(table18.first_page, PageIndex(37));
        assert_eq!(table18.last_page, PageIndex(38));
        assert_eq!(table18.empty_candidate, 45);
        assert_eq!(db.get_header().next_page_sequence, 6);

        {
            let table18_page = db.load_page(table18.last_page)?;
            assert_eq!(table18_page.header.next_page, PageIndex(45));
            assert_eq!(table18_page.header.page_sequence, 5);

            let PageContent::Data(data) = &table18_page.content else {
                panic!("table 18 should have a data page after seeding");
            };
            assert_eq!(data.header.transaction_row_count, 0);
            assert_eq!(
                data.row_groups
                    .iter()
                    .map(|group| (group.row_presence_flags, group.transaction_row_flags))
                    .collect::<Vec<_>>(),
                vec![(0xffff, 0x0000), (0x0001, 0x0000)]
            );
            assert_eq!(
                data.rows
                    .values()
                    .map(|row| *row.as_variant::<Unknown18Row>().expect("table 18 row type"))
                    .collect::<Vec<_>>(),
                DEFAULT_UNKNOWN18_ROWS
            );
        }

        let history_page = db.load_page(PageIndex(40))?;
        assert_eq!(history_page.header.used_size, 32);
        assert_eq!(history_page.header.free_size, 4018);

        Ok(())
    }
}
