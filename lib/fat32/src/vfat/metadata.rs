use core::fmt;

use alloc::string::String;

use crate::traits;
use crate::vfat::dir::VFatRegularDirEntry;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(pub u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(pub u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(pub u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub time: Time,
    pub date: Date,
}

/// Metadata for a directory entry.
/// TODO: actually use this. this shouldn't directly map to disk, and should
/// be used for convenience
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    attributes: Attributes,
    create_timestamp: Timestamp,
    last_accessed_date: Date,
    last_modification_timestamp: Timestamp,
}

impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        (self.date.0 >> 9 & 0b1111111) as usize + 1980
    }

    fn month(&self) -> u8 {
        (self.date.0 >> 5 & 0b1111) as u8
    }

    fn day(&self) -> u8 {
        (self.date.0 & 0b11111) as u8
    }

    fn hour(&self) -> u8 {
        (self.time.0 >> 11 & 0b11111) as u8
    }

    fn minute(&self) -> u8 {
        (self.time.0 >> 5 & 0b111111) as u8
    }

    fn second(&self) -> u8 {
        (self.time.0 & 0b11111) as u8 * 2
    }
}

impl Metadata {
    pub fn from(dir_entry: VFatRegularDirEntry) -> Metadata {
        Metadata {
            attributes: dir_entry.attributes,
            create_timestamp: dir_entry.create_timestamp,
            last_accessed_date: dir_entry.last_accessed_date,
            last_modification_timestamp: dir_entry.last_modification_timestamp,
        }
    }
}

impl traits::Metadata for Metadata {
    type Timestamp = Timestamp;

    fn read_only(&self) -> bool {
        self.attributes.0 & 0b1 != 0
    }

    fn hidden(&self) -> bool {
        self.attributes.0 & 0b10 != 0
    }

    fn created(&self) -> Timestamp {
        self.create_timestamp
    }

    fn accessed(&self) -> Timestamp {
        Timestamp {
            time: Time(0),
            date: self.last_accessed_date,
        }
    }

    fn modified(&self) -> Timestamp {
        self.last_modification_timestamp
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
