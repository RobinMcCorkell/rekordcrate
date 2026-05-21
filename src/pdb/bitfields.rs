// Copyright (c) 2026 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Packed fields/bitfields used in PDB files.

// This is necessary since `bitfield` generates methods that otherwise violate this lint.
#![allow(clippy::must_use_candidate)]

use binrw::{BinRead, BinWrite};
use modular_bitfield::prelude::*;

use crate::pdb::RowGroup;

/// Packed field found in the page header containing:
/// - number of used row offsets in the page (13 bits).
/// - number of valid rows in the page (11 bits).
#[bitfield]
#[derive(BinRead, BinWrite, Debug, PartialEq, Eq, Clone, Copy)]
#[br(little, map = Self::from_bytes)]
#[bw(little, map = |x: &PackedRowCounts| x.into_bytes())]
pub struct PackedRowCounts {
    pub num_rows: B13,
    pub num_rows_valid: B11,
}

impl Default for PackedRowCounts {
    fn default() -> Self {
        Self::new()
    }
}

impl PackedRowCounts {
    /// Create a `PackedRowCounts` assuming all rows in the page are valid,
    /// e.g. when we are serializing a page without any deleted rows.
    pub fn from_all_valid(num_rows: usize) -> Self {
        Self::new()
            .with_num_rows(num_rows as u16)
            .with_num_rows_valid(num_rows as u16)
    }

    /// Get the number of row groups in the page.
    pub(crate) fn num_row_groups(&self) -> u16 {
        self.num_rows().div_ceil(RowGroup::MAX_ROW_COUNT as u16)
    }

    /// Get the index of the last row group in the page and
    /// the index of the last row in that row group.
    pub(crate) fn last_row_index(&self) -> Option<(u16, u16)> {
        let rgi = self.num_row_groups().checked_sub(1)?;
        let rsi = self
            .num_rows()
            .checked_sub(1)
            .map(|n| n % RowGroup::MAX_ROW_COUNT as u16)?;
        Some((rgi, rsi))
    }

    /// Increment the number of rows e.g. when we allocate a row in the page.
    pub(crate) fn increment_num_rows(&mut self) {
        self.set_num_rows(self.num_rows() + 1);
    }

    // Increment the number of valid rows e.g. when we insert a row into the page.
    pub(crate) fn increment_num_rows_valid(&mut self) {
        self.set_num_rows_valid(self.num_rows_valid() + 1);
    }
}

/// Page flags stored in the page header.
///
/// Note that `modular-bitfield` stores the bits in LSB-first order so the
/// bitfield definition is reversed compared to typical notation.
#[bitfield(bits = 8)]
#[derive(BinRead, BinWrite, Debug, PartialEq, Eq, Clone, Copy)]
#[br(little, map = Self::from_bytes)]
#[bw(little, map = |x: &Self| x.into_bytes())]
pub struct PageFlags {
    /// Unknown flag that is unset in all known exports.
    pub unknown0: bool,
    /// Unknown flag that is unset in all known exports.
    pub unknown1: bool,
    /// Unknown flag that is set in all known exports.
    pub unknown2: bool,
    /// Unknown flag that is unset in all known exports.
    pub unknown3: bool,
    /// Set when a data page contains deleted or otherwise invalid rows.
    pub has_deleted_rows: bool,
    /// Unknown flag that is set in all known exports.
    pub unknown5: bool,
    /// Set when the page contains free-space entries rather than table rows.
    pub is_free_space_page: bool,
    /// Unknown flag that is unset in all known exports.
    pub unknown7: bool,
}

impl Default for PageFlags {
    fn default() -> Self {
        Self::new()
            .with_unknown0(false)
            .with_unknown1(false)
            .with_unknown2(true)
            .with_unknown3(false)
            .with_has_deleted_rows(false)
            .with_unknown5(true)
            .with_is_free_space_page(false)
            .with_unknown7(false)
    }
}

impl PageFlags {
    /// Create a `PageFlags` for a typical data page.
    pub fn new_data_page() -> Self {
        Self::default().with_is_free_space_page(false)
    }

    /// Create a `PageFlags` for a typical free-space page.
    pub fn new_free_space_page() -> Self {
        Self::default().with_is_free_space_page(true)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_page_flags_free_space() {
        let flags = PageFlags::new_free_space_page();
        assert_eq!(flags.into_bytes(), [0x64]);
    }

    #[test]
    fn test_page_flags_data() {
        let mut flags = PageFlags::new_data_page();
        assert_eq!(flags.into_bytes(), [0x24]);

        flags.set_has_deleted_rows(true);
        assert_eq!(flags.into_bytes(), [0x34]);
    }
}
