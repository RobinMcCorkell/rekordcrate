// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Tests that all structural invariants hold for real rekordbox-generated PDB files.

use rekordcrate::pdb::io::Database;
use rekordcrate::pdb::validation::validate;
use rekordcrate::pdb::DatabaseType;
use std::io::Cursor;

fn assert_no_validation_errors(data: &[u8], name: &str) {
    let cursor = Cursor::new(data);
    let mut db = Database::open_non_persistent(cursor, DatabaseType::Plain)
        .unwrap_or_else(|e| panic!("failed to open {name}: {e}"));
    let errors = validate(&mut db);
    assert!(
        errors.is_empty(),
        "{name} has validation errors:\n{errors:#?}"
    );
}

macro_rules! pdb_validation_test {
    ($name:ident, $path:expr) => {
        #[test]
        fn $name() {
            let data = include_bytes!(concat!("../data/", $path));
            assert_no_validation_errors(data, $path);
        }
    };
}

pdb_validation_test!(
    validate_complete_export_empty,
    "complete_export/empty/PIONEER/rekordbox/export.pdb"
);
pdb_validation_test!(
    validate_complete_export_demo_tracks,
    "complete_export/demo_tracks/PIONEER/rekordbox/export.pdb"
);
pdb_validation_test!(
    validate_complete_export_device_library_plus,
    "complete_export/device_library_plus/PIONEER/rekordbox/export.pdb"
);
pdb_validation_test!(validate_incremental_000, "incremental/000/export.pdb");
pdb_validation_test!(validate_incremental_016, "incremental/016/export.pdb");
pdb_validation_test!(validate_incremental_063, "incremental/063/export.pdb");
pdb_validation_test!(
    validate_incremental_063_removed_one,
    "incremental/063-removed-one/export.pdb"
);
pdb_validation_test!(validate_incremental_100, "incremental/100/export.pdb");
pdb_validation_test!(validate_incremental_196, "incremental/196/export.pdb");
pdb_validation_test!(validate_incremental_292, "incremental/292/export.pdb");
pdb_validation_test!(
    validate_incremental_292_removed_one,
    "incremental/292-removed-one/export.pdb"
);
pdb_validation_test!(
    validate_incremental_big_000,
    "incremental-big/000/export.pdb"
);
pdb_validation_test!(
    validate_incremental_big_009,
    "incremental-big/009/export.pdb"
);
pdb_validation_test!(validate_pdb_num_rows, "pdb/num_rows/export.pdb");
