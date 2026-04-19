# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### Bug Fixes

- pdb: Improve DeviceSQLString error handling
- pdb: Remove wrong `unknown8` field from `Page` struct
- pdb: Use 8-bit color indices consistently
- pdb: Fix offset read logic for `name` in `Artist` rows
- pdb: Use correct error position when writing track rows
- pdb: Enforce correct endianness when writing track rows
- pdb: Fix album string reading
- pdb: Fix order of playlist tree node `id` and `sort_order` fields
- pdb: Skip reading rows of invalid pages
- pdb: Adhere to rows alignment to type alignment when writing
- Always read all 16 rows potential from each row group
- Plain Hotcues being rejected due to cuetype mismatch
- pdb: Change padding logic for track rows
- pdb: Pass parsing stage of track_page test
- pdb: Apply review feedback for 3d5d57c
- pdb: Validate row count in RowGroup to avoid silent overflow
- pdb: Allow unused method to pass CI
- pdb: Pass track_page test by not reversing in present_rows
- pdb: Re-add comments removed during merge
- pdb: Pass 5 more test_pdb_num_rows* tests
- Doctest broken by previous string refactor commit
- pdb: All tests pass + clean up comments
- pdb: Apply review feedback for 78ee51c
- pdb: Accept review feedback for 969d507
- `genres_page` test offset padding and rowgroup ordering
- pdb: WIP pass artists_page test
- Add `ofs_name` field to Album row and correctly (de-)serialize it
- pdb: Set fixed 4-byte alignment on labels
- pdb: Set 4-byte alignment for Artwork rows
- pdb: Increase struct fields visibility in ext.rs for use in tests
- pdb: Add padding field to tag struct
- pdb: Set fixed 4-byte alignment for tag rows
- pdb: Add mapping for optional NonZero<u32> in ParentId struct
- pdb: Explicit pattern matching to avoid extra `clone()`
- pdb: Custom `BinWrite` for `IndexPageContent` to comply with binary
- pdb: Address feedback by @Swiftb0y
- pdb: Only allow PageIndex values less than 0x03FF_FFFF
- Fix various compiler or clippy warnings with Rust 1.90
- device: Load_settings, don't fail if settings files are missing
- Build status badge/shield

### Documentation

- contributing: Add contribution guide
- readme: Fix file paths in CLI section
- readme: Add documentation for `rekordbox-setting` binary
- pdb: Add TODO comment regarding usage of `u16::div_ceil`
- pdb: Add documentation comments for temporary fields, too
- changelog: Fix typo
- pdb: Add doc comments for struct fields in ext.rs
- Link to upstream Tag row docs in docs
- pdb: Enhance documentation for some index page fields
- Fix comment for Track enum variant

### Features

- pdb: Add binrw-based DeviceSQLString impl
- pdb: Add DeviceSQLString validation failure tests
- setting: Implement `Default` trait for all data structs
- setting: Calculate CRC16 checksum automatically
- setting: Add initial payload parser for `DEVSETTING.DAT` files
- setting: Add support for "Waveform color" field
- setting: Add support for "Waveform Current Position" field
- setting: Add support for "Overview Waveform Type" field
- setting: Add support for "Key display format" field
- pdb: Improve readability of Debug format for `DeviceSQLString`
- pdb: Add preliminary support for parsing `*.pdb` files with binrw
- pdb: Implement support for serialization of Artist rows
- pdb: Implement support for serialization of Track rows
- cli: Add better command line interface using clap
- pdb: Add convenience function to check if tree node is folder
- cli: Add subcommand to print playlist tree
- setting: Add method to construct default setting objects
- setting: Derive `Clone` and `Copy` traits for all setting values
- setting: Derive `Clone` for all setting data structs
- setting: Add `Display` implementation for setting values
- pdb: Add Columns table
- pdb: Mark table rows as serializable
- pdb: Implement support for serialization of table pages
- xml: Add support for Rekordbox XML format
- Use "clean" buffer in `track_page` test & refactor `DeviceSQLString` construction
- Add `artist_page_long` test to cover the 0x64 artist subtype
- Get `labels_page` passing
- Add `VarOffsetTail` as an abstraction for trailing var-len data
- Replace `VarOffsetTail` with `OffsetArray` and add ExplicitPadding
- Parse rekordbox exportExt.pdb format
- Convert dump-ext-pdb into flag instead with some guessing magic
- pdb: Initial index page implementation
- pdb: Add methods to create new and empty `IndexEntry` instances
- pdb: Parse MenuItem rows (table 17)
- Also print playlist content (artist - title) for list-playlists
- device: Add preliminary high-level API for reading settings
- cli: Add command to list settings from device export
- device: Add methods to read playlist entries and tracks
- cli: Add command to export playlists as M3U
- pdb: Add History row type

### Refactor

- pdb: Unify the two different `DeviceSQLString` implementations
- pdb: Add support for parsing `Row` enum using binrw
- pdb: Add support for parsing `Page` struct using binrw
- pdb: Add support for parsing `RowGroup` struct with binrw
- pdb: Replace nom with binrw in most of  `rekordcrate-pdb`
- pdb: Add support for parsing page indices with binrw
- pdb: Remove nom parser implementation in favor of binrw
- pdb: Add `PageIndex::offset()` method
- pdb: Mark `Page` struct and `Row`  enum as not serializable
- util: Remove remaining  `ColorIndex` code that uses `u16`
- pdb: Use `Row` enum tuples with struct instead of enum structs
- pdb: Don't parse row groups lazily
- setting: Enum `#[derive(Default)]` instead of `impl Default`
- Fix clippy warnings for rust 1.64
- Update to `binrw` 0.10
- pdb: Add dedicated ID field types for extra type-safety
- cli: Return `Result` from main method instead of unwrapping
- Avoid temporary Vec allocation in assert_pdb_row_count
- Use div_ceil instead of handrolled checked arithmatic
- Improve `BinRead` impl of `RowGroup`
- Simplify `Page` and `RowGroup` parsing
- Fix test `assert_eq!(result, expected)` parameter order
- Move `Row::Artist`-specific padding to `Artist` struct
- Remove length `assert_eq` from roundtrip tests
- Use `VarOffsetTail` for Artist rows
- Use `VarOffsetTail` for `Album` struct
- Move `VarOffsetTail` to its own module
- Add separate row `Subtype` type
- Remodel OffsetArray so it can be used with (almost) any type
- Use OffsetArray for Track rows
- Cleanup `pdb/offset_array.rs` a little by reducing duplication
- OffsetArray->OffsetArrayContainer, OffsetArrayImpl->OffsetArray
- Outline page test buffers
- pdb: Prepare for index page implementation
- pdb: Address feedback from @Swiftb0y
- pdb: Address NITs from @Swiftb0y
- pdb: Rename get_* methods to into_* to align with `std`
- pdb: `#[br(temp, assert/#[bw(calc` -> `#[brw(magic` in structs
- pdb: Remove unused methods from `Page` impl
- pdb: Address NITs from @Holzhaus and make clippy happy
- pdb: Remove unneeded explicit padding from tag rows
- Optimize impl fmt::Display for DeviceSQLString
- pdb: Rename MenuItem to Menu and add MenuVisibility enum
- device: Add support for PDB and move code from `main`
- device: Expose Pdb in DeviceExport and remove wrappers
- device: Return track references instead of clones
- cli: Use Pdb abstraction for playlist listing
- Implement Display for Settings to reduce some duplication

### Testing

- Auto-generate pdb smoke tests from data directory
- Auto-generate anlz smoke tests from data directory
- Allow snake_case names in tests_pdb.rs.in
- Add tests for individual `MYSETTING.DAT` options
- Add tests for individual `MYSETTING2.DAT` options
- Add tests for individual `DJMMYSETTING.DAT` options
- Update PDB tests to use binrw implementation instead of nom
- pdb: Add some simple roundtrip tests for PDB header
- pdb: Add roundtrip tests for label, key and color rows
- pdb: Use more appropriate DeviceSQLString ctor in track row test
- util: Add helper function for passing args to roundtrip tests
- util: Add additional length checks to roundtrip tests
- util: Print useful diffs when `assert_eq!` fails on large blobs
- Add regression tests to ensure all rows are read
- pdb: Add genres_page test
- pdb: Add artists_page test
- pdb: Move tests to sepparate file
- pdb: Fix mistake in artists_page test
- pdb: Corrections after moving tests
- pdb: WIP added albums_page test
- pdb: Add labels_page test
- pdb: Add keys_page test
- pdb: Add colors_page test and fix colors padding to pass test
- pdb: Add playlist entry row and page tests
- pdb: Add playlist tree row and page tests
- pdb: Add artwork page test
- pdb: Add tag and track_tag page tests
- pdb: Add history playlists and entries page tests
- pdb: Add `index_page` unit test
- pdb: Add tests for Menu row and page
- pdb: Add tests for History row

### Anlz

- ANLZ files: Add basic ANLZ parser
- ANLZ files: Move Content parser into separate method
- ANLZ files: Add missing `ContentKind` values for `Cue`/`ExtendedCue`
- ANLZ files: Add parser for BeatGrid sections
- ANLZ files: Add parser for CueList sections
- ANLZ files: Add parser for ExtendedCueList sections
- ANLZ files: Add parser for Path sections
- ANLZ files: Add parser for VBR sections
- ANLZ files: Add parser for WaveformPreview sections
- ANLZ files: Add parser for TinyWaveformPreview sections
- ANLZ files: Implement missing comment parser for `ExtendedCue` sections
- ANLZ files: Handle errors when decoding strings
- ANLZ files: Remove unnecessary len_comment member from `ExtendedCue`
- ANLZ files: Handle errors when converting size/length fields
- ANLZ files: Add parser for WaveformDetail sections
- ANLZ files: Add parser for WaveformColorPreview sections
- ANLZ files: Add parser for WaveformColorDetail sections
- ANLZ files: Add parser for SongStructure sections
- ANLZ files: Check if `len_entry_bytes` has expected value
- ANLZ files: Fix `ExtendedCue` color table in comment
- ANLZ files: Document that these files also exist when not using device exports
- ANLZ files: Ensure that `len_entry_bytes` fields have expected value
- ANLZ files: Use `binrw` instead of `nom` to parse `ANLZ*.DAT` files
- ANLZ files: Split Content enum into separate structs
- ANLZ files: Use `NullWideString` for `UTF16-BE` strings instead of `Vec<u8>`
- ANLZ files: Add basic serialization support
- ANLZ files: Explain why unknown values are allowed in some enums
- ANLZ files: Remove `pub` visiblity from internal fields
- ANLZ files: Add assertions for section lengths using header content size
- ANLZ files: Mark length fields as `#[br(temp)]`
- ANLZ files: Rename `data` field of Song Structure tag to `phrases`
- ANLZ files: Add detection if song stucture section is encrypted
- ANLZ files: Move potentially encrypted song structure data into separate struct
- ANLZ files: Add support for encrypted data in song structure sections
- ANLZ files: Improve `SongStructureData` encryption detection
- ANLZ files: Remove `Unknown` variants from `Mood` and `Bank` enums
- ANLZ files: Fix typo in `SongStructure` struct doc comment

### Bin

- ANLZ files: Add tool to dump anlz files

### Build

- SETTING.DAT files: Generate smoke tests for setting files

### Lib.rs

- SETTING.DAT files: Add linter settings and remove sample code

### Pdb

- PDB files: Add initial parser for PDB file headers
- PDB files: Add preliminary parser for pages
- PDB files: Implement basic row parser
- PDB files: Add parser for DeviceSQL strings
- PDB files: Add parser for Color rows
- PDB files: Implement a way to parse the color from u8 instead of u16
- PDB files: Add a parser for Album rows
- PDB files: Add a parser for Artist rows
- PDB files: Add a parser for Track rows
- PDB files: Add a parser for Artwork rows
- PDB files: Add a parser for Genre rows
- PDB files: Add a parser for Key rows
- PDB files: Add a parser for Label rows
- PDB files: Add a parser for HistoryPlaylist rows
- PDB files: Add a parser for HistoryEntry rows
- PDB files: Add a parser for PlaylistTreeNode rows
- PDB files: Add a parser for PlaylistEntry rows
- PDB files: Handle string decode errors properly
- PDB files: Add error handling for conversions to usize
- PDB files: Simplify PageIndexIterator and avoid unnecessary page retrieval

### Pdb.rs

- PDB files: Fix Table RowGroup parsing
- PDB files: Fix Row::parse_album string offset
- PDB files: Implement proper PageIndex iteration
- PDB files: Fix `DeviceSQLString::parse_long_utf16le`
- PDB files: Improve utf16 sanity check explanation
- PDB files: Fix `DeviceSQLString::parse_long_ascii`
- PDB files: Implement ISRC-string parsing

### Setting

- SETTING.DAT files: Parse CRC16 checksum
- SETTING.DAT files: Add proper error handling for usize conversions
- SETTING.DAT files: Rename `unknown1` field to `data`
- SETTING.DAT files: Add `SettingData` enum for different data types
- SETTING.DAT files: Add parser for "PLAYER > DJ SETTING" options in `MYSETTING.DAT` files
- SETTING.DAT files: Add parser for "PLAYER > DJ SETTING" options in `MYSETTING2.DAT` files
- SETTING.DAT files: Add parser for "PLAYER > DISPLAY" options in `MYSETTING(2).DAT` files
- SETTING.DAT files: Add support for fader curve settings in `DJMMYSETTING.DAT`
- SETTING.DAT files: Use compact nullbyte slice notation
- SETTING.DAT files: Add support for other "DJ SETTING" options in `DJMMYSETTING.DAT`
- SETTING.DAT files: Add support for "MIXER > BRIGHTNESS" options in `DJMMYSETTING.DAT`
- SETTING.DAT files: Use `binrw` instead of `nom` to parse `*SETTING.DAT` files
- SETTING.DAT files: Add PartialEq for all setting types
- SETTING.DAT files: Add basic serialization support
- SETTING.DAT files: Use `repr` instead of `magic` to map enum values
- SETTING.DAT files: Rename `company` to `brand` and document possible values.
- SETTING.DAT files: Fix wrong order of JogMode enumeration byte values
- SETTING.DAT files: Fix wrong order of HeadphonesMonoSplit enumeration byte values
- SETTING.DAT files: Remove wrong PadButtonBrightness enum value
- SETTING.DAT files: Move data section contents into separate structs
- SETTING.DAT files: Remove `pub` visiblity from internal fields
- SETTING.DAT files: Add assertion for string lengths in string section
- SETTING.DAT files: Mark `len_stringdata` field as temp
- SETTING.DAT files: Use fixed size array instead of `Vec<u8>` for DevSetting variant
- SETTING.DAT files: Mark `len_data` field as temp
- SETTING.DAT files: Add TODO regarding checksum field

### Setting/anlz

- ANLZ files: Move `#[binrw]` above `#[derive(..)]`

## [0.2.0] - 2022-10-09

- Switch from `nom` to `binrw` to pave the way for serialization support in the future
- Improve documentation and add contribution guide
- Add new command-line interface, which now also offers a way list the playlist tree
- Add support for reading and serializing `*SETTING.DAT` files
- Add more tests
- Various small bug fixes and improvements (e.g., using 8-bit color indices consistently)

## [0.1.0] - 2022-02-10

Initial release.
