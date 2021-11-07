use core::fmt;

use alloc::string::String;

use crate::traits;

/// A date as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Date(u16);

/// Time as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Time(u16);

/// File attributes as represented in FAT32 on-disk structures.
#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Attributes(u8);

/// A structure containing a date and time.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub time: Time,
    pub date: Date,
}

/// Metadata for a directory entry.
#[derive(Default, Debug, Clone)]
pub struct Metadata {
    name: [u8; 8],
    extension: [u8; 3],
    attributes: Attributes,
    windows_nt_reserved: u8,
    creation_time_tenth_seconds: u8,
    create_timestamp: Timestamp,
    last_accessed_date: Date,
    first_cluster_high_16: u16,
    last_modification_timestamp: Timestamp,
    first_cluster_low_16: u16,
    size: u32,
}

impl traits::Timestamp for Timestamp {
    fn year(&self) -> usize {
        (self.date.0 & (0b1111111 << 9)) as usize
    }

    fn month(&self) -> u8 {
        (self.date.0 & (0b1111 << 5)) as u8
    }

    fn day(&self) -> u8 {
        (self.date.0 & 0b11111) as u8
    }

    fn hour(&self) -> u8 {
        (self.time.0 & (0b11111 << 11)) as u8
    }

    fn minute(&self) -> u8 {
        (self.time.0 & (0b1111111 << 5)) as u8
    }

    fn second(&self) -> u8 {
        (self.time.0 & 0b11111) as u8 * 2
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
        write!(f, "{:?}.{:?}", self.name, self.extension)
    }
}
