use core::panic;
use rekordcrate::pdb::{Database, DatabaseType, PageType, PlainPageType, PlainRow, Row};
use std::{io::Cursor, path::PathBuf};

// Set REKORDCRATE_TEST_DUMP_PATH to dump modified databases to that directory for inspection.

fn get_table_row_count(
    db: &mut Database<'_, impl std::io::Read + std::io::Seek>,
    page_type: PageType,
) -> usize {
    let table_id = db
        .get_header()
        .get_table_for_type(page_type)
        .expect("Failed to get table by page type");
    println!(
        "Loading pages for page type {:?} in table: {:?}",
        page_type, table_id
    );
    let page_ids = db
        .load_pages_for_table(table_id)
        .expect("Failed to load pages for table");
    println!("Table {:?} has page IDs: {:?}", table_id, page_ids);

    page_ids
        .into_iter()
        .filter_map(|page_id| {
            println!("Loading page: {:?}", page_id);
            db.load_page(page_id)
                .expect("failed to load page")
                .content
                .as_data()
                .map(|data_content| {
                    data_content
                        .row_groups
                        .iter()
                        .map(|row_group| row_group.len())
                        .sum::<usize>()
                })
        })
        .sum()
}

fn assert_pdb_modify_verify(
    test_name: &str,
    modify: impl FnOnce(&mut Database<'_, Cursor<&mut [u8]>>),
    verify: impl FnOnce(&mut Database<'_, Cursor<&[u8]>>),
) {
    let mut data = Vec::from(include_bytes!("../data/pdb/num_rows/export.pdb"));
    let mut io = Cursor::new(data.as_mut_slice());
    println!("Opening database for modification");
    let mut db = Database::open(&mut io, DatabaseType::Plain).expect("Failed to open database");

    println!("Modifying database");
    modify(&mut db);
    println!("Closing database");
    db.close().expect("failed to close database");
    drop(io);

    if let Some(save_dir) = std::env::var("REKORDCRATE_TEST_DUMP_PATH")
        .ok()
        .map(|s| PathBuf::from(s))
    {
        let save_subdir = save_dir.join("test_pdb_write").join(test_name);
        std::fs::create_dir_all(&save_subdir).expect("failed to create dump directory");
        let save_path = save_subdir.join("export.pdb");
        println!("Dumping database for introspection: {:?}", save_path);
        std::fs::write(save_path, &data).expect("failed to dump modified test database");
    }

    let mut io = Cursor::new(data.as_slice());
    println!("Opening database for verification");
    let mut db = Database::open_non_persistent(&mut io, DatabaseType::Plain)
        .expect("Failed to open database");

    println!("Verifying database");
    verify(&mut db);
}

#[test]
fn test_pdb_no_loaded_pages() {
    assert_pdb_modify_verify(
        "no_loaded_pages",
        |_| {},
        |db| {
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Albums)),
                2226
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Artists)),
                2216
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Artwork)),
                2178
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Colors)),
                8
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Genres)),
                315
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::HistoryPlaylists)),
                1
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::HistoryEntries)),
                73
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Keys)),
                67
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Labels)),
                688
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::PlaylistTree)),
                104
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::PlaylistEntries)),
                6637
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Columns)),
                27
            );
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Tracks)),
                3886
            );
        },
    );
}

#[test]
fn test_pdb_unchanged_table() {
    assert_pdb_modify_verify(
        "unchanged_table",
        |db| {
            let table_id = db
                .get_header()
                .get_table_for_type(PageType::Plain(PlainPageType::Tracks))
                .unwrap();
            println!("Loading pages for table: {:?}", table_id);
            db.load_pages_for_table(table_id)
                .expect("failed to load pages for tracks table");
        },
        |db| {
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Tracks)),
                3886
            );
        },
    );
}

#[test]
fn test_pdb_modify_tracks() {
    assert_pdb_modify_verify(
        "modify_tracks",
        |db| {
            let table_id = db
                .get_header()
                .get_table_for_type(PageType::Plain(PlainPageType::Tracks))
                .expect("Failed to get table by page type");
            let page_ids = db
                .load_pages_for_table(table_id)
                .expect("Failed to load pages for table");

            for page_id in page_ids {
                let page = db.load_page(page_id).expect("failed to load page");

                if let Some(data_content) = page.content.as_data_mut() {
                    for row_group in data_content.row_groups.iter_mut() {
                        for row in row_group {
                            match row {
                                Row::Plain(PlainRow::Track(track)) => {
                                    // Set the rating of all tracks to 5 stars.
                                    track.rating = 5;
                                }
                                _ => panic!("encountered non-track row in tracks table"),
                            }
                        }
                    }
                }
            }
        },
        |db| {
            assert_eq!(
                get_table_row_count(db, PageType::Plain(PlainPageType::Tracks)),
                3886
            );
            let table_id = db
                .get_header()
                .get_table_for_type(PageType::Plain(PlainPageType::Tracks))
                .expect("Failed to get table by page type");
            let page_ids = db
                .load_pages_for_table(table_id)
                .expect("Failed to load pages for table");
            for page_id in page_ids {
                let page = db.load_page(page_id).expect("failed to load page");

                if let Some(data_content) = page.content.as_data() {
                    for row_group in data_content.row_groups.iter() {
                        for row in row_group {
                            match row {
                                Row::Plain(PlainRow::Track(track)) => {
                                    assert_eq!(
                                        track.rating, 5,
                                        "track rating was not modified correctly"
                                    );
                                }
                                _ => panic!("encountered non-track row in tracks table"),
                            }
                        }
                    }
                }
            }
        },
    );
}
