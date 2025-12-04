// Copyright (c) 2025 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Common types used in multiple modules.

use crate::pdb::string::StringError;
use binrw::binrw;
use thiserror::Error;

/// Enumerates errors returned by this library.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RekordcrateError {
    /// Represents a failure to decode a DeviceSQL string.
    #[error(transparent)]
    StringError(#[from] StringError),

    /// Represents a failure to parse input.
    #[error(transparent)]
    ParseError(#[from] binrw::Error),

    /// Represents an `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// Represents a failure to handle the PDB database.
    #[error(transparent)]
    PdbError(#[from] crate::pdb::PdbError),
}

/// Type alias for results where the error is a `RekordcrateError`.
pub type RekordcrateResult<T> = std::result::Result<T, RekordcrateError>;

/// Indexed Color identifiers used for memory cues and tracks.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ColorIndex {
    /// No color.
    #[brw(magic = 0u8)]
    None,
    /// Pink color.
    #[brw(magic = 1u8)]
    Pink,
    /// Red color.
    #[brw(magic = 2u8)]
    Red,
    /// Orange color.
    #[brw(magic = 3u8)]
    Orange,
    /// Yellow color.
    #[brw(magic = 4u8)]
    Yellow,
    /// Green color.
    #[brw(magic = 5u8)]
    Green,
    /// Aqua color.
    #[brw(magic = 6u8)]
    Aqua,
    /// Blue color.
    #[brw(magic = 7u8)]
    Blue,
    /// Purple color.
    #[brw(magic = 8u8)]
    Purple,
}

/// Track file type.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FileType {
    /// Unknown file type.
    #[brw(magic = 0x0u16)]
    Unknown,
    /// MP3.
    #[brw(magic = 0x1u16)]
    Mp3,
    /// M4A.
    #[brw(magic = 0x4u16)]
    M4a,
    /// FLAC.
    #[brw(magic = 0x5u16)]
    Flac,
    /// WAV.
    #[brw(magic = 0xbu16)]
    Wav,
    /// AIFF.
    #[brw(magic = 0xcu16)]
    Aiff,
    /// Value that we haven't seen before.
    Other(u16),
}

#[cfg(test)]
pub(crate) mod testing {
    use binrw::{
        meta::{ReadEndian, WriteEndian},
        prelude::*,
        Endian,
    };
    use pretty_assertions::assert_eq;
    use pretty_hex::pretty_hex;

    macro_rules! assert_eq_hex {
        ($cond:expr, $expected:expr) => {
            assert_eq!(pretty_hex($cond), pretty_hex($expected));
        };
    }
    pub fn test_roundtrip_with_args<'a, T>(
        bin: &[u8],
        obj: T,
        read_args: <T as binrw::BinRead>::Args<'a>,
        write_args: <T as binrw::BinWrite>::Args<'a>,
    ) where
        <T as binrw::BinRead>::Args<'a>: Clone,
        <T as binrw::BinWrite>::Args<'a>: Clone,
        T: BinRead + BinWrite + PartialEq + core::fmt::Debug + ReadEndian + WriteEndian,
    {
        test_read_with_args(bin, &obj, read_args);
        test_write_with_args(bin, &obj, write_args);
    }

    pub fn test_read_with_args<'a, T>(
        bin: &[u8],
        obj: &T,
        read_args: <T as binrw::BinRead>::Args<'a>,
    ) where
        <T as binrw::BinRead>::Args<'a>: Clone,
        T: BinRead + PartialEq + core::fmt::Debug + ReadEndian,
    {
        let endian = Endian::NATIVE;
        let mut cursor = binrw::io::Cursor::new(bin);
        let parsed = T::read_options(&mut cursor, endian, read_args.clone()).unwrap();
        assert_eq!(&parsed, obj);
    }

    pub fn test_write_with_args<'a, T>(
        bin: &[u8],
        obj: &T,
        write_args: <T as binrw::BinWrite>::Args<'a>,
    ) where
        <T as binrw::BinWrite>::Args<'a>: Clone,
        T: BinWrite + PartialEq + core::fmt::Debug + WriteEndian,
    {
        let endian = Endian::NATIVE;
        let mut writer = binrw::io::Cursor::new(Vec::with_capacity(bin.len()));
        obj.write_options(&mut writer, endian, write_args.clone())
            .unwrap();
        assert_eq_hex!(&writer.get_ref(), &bin);
    }

    pub fn test_write_then_read_with_args<'a, T>(
        obj: &T,
        read_args: <T as binrw::BinRead>::Args<'a>,
        write_args: <T as binrw::BinWrite>::Args<'a>,
    ) where
        <T as binrw::BinRead>::Args<'a>: Clone,
        <T as binrw::BinWrite>::Args<'a>: Clone,
        T: BinRead + BinWrite + PartialEq + core::fmt::Debug + ReadEndian + WriteEndian,
    {
        let endian = Endian::NATIVE;
        let mut bin = Vec::new();
        let mut writer = binrw::io::Cursor::new(&mut bin);
        obj.write_options(&mut writer, endian, write_args.clone())
            .unwrap();
        test_read_with_args(&bin, obj, read_args);
    }

    pub fn test_roundtrip<'a, T>(bin: &[u8], obj: T)
    where
        <T as binrw::BinRead>::Args<'a>: Default + Clone,
        <T as binrw::BinWrite>::Args<'a>: Default + Clone,
        T: BinRead + BinWrite + PartialEq + core::fmt::Debug + ReadEndian + WriteEndian,
    {
        test_roundtrip_with_args(
            bin,
            obj,
            <T as binrw::BinRead>::Args::default(),
            <T as binrw::BinWrite>::Args::default(),
        );
    }
}
