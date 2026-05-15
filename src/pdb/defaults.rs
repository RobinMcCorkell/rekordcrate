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
use super::{Color, ColumnEntry, History, Key, KeyId, Menu, MenuVisibility, PlainRow, Row};
use crate::util::ColorIndex;
use crate::Result;

/// Rekordbox's history row version string for exported PDB files.
pub const DEFAULT_HISTORY_VERSION: &str = "1000";

/// Insert the rows present in a freshly exported blank `export.pdb`.
///
/// This mirrors `data/incremental/000/export.pdb`: colors, metadata columns,
/// menu definitions, and a single history sync row. It intentionally does not
/// insert any key rows because Rekordbox leaves that table empty in a blank
/// export.
pub fn insert_initial_content<IO: Read + Write + Seek>(
    db: &mut Database<IO>,
    history_date: DeviceSQLString,
    history_label: DeviceSQLString,
) -> Result<()> {
    insert_standard_colors(db)?;
    insert_default_columns(db)?;
    insert_default_menu_rows(db)?;
    insert_default_history_row(db, history_date, history_label)?;
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::pdb::DatabaseType;
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

        insert_standard_keys(&mut db)?;
        assert_eq!(db.iter_rows::<Key>()?.count()?, 24);

        Ok(())
    }
}
