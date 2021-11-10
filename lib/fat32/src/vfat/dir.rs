use core::marker::PhantomData;
use core::str::from_utf8;

use alloc::string::String;
use alloc::vec::Vec;

use shim::const_assert_size;
use shim::ffi::OsStr;
use shim::io;
use shim::newioerr;

use crate::traits;
use crate::util::VecExt;
use crate::vfat::{Attributes, Date, Metadata, Time, Timestamp};
use crate::vfat::{Cluster, Entry, FatEntry, File, VFatHandle};

const LONG_FILENAME_MARKER: u8 = 0xF;

#[derive(Clone, Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    vfat: HANDLE,
    cluster: Cluster,
    name: String,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
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

const_assert_size!(VFatRegularDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatLfnDirEntry {
    sequence_number: u8,
    name_first: [u16; 5],
    attributes: Attributes,
    vfat_type: u8,
    checksum: u8,
    name_second: [u16; 6],
    zeroes: [u8; 2],
    name_third: [u16; 2],
}

const_assert_size!(VFatLfnDirEntry, 32);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatUnknownDirEntry {
    _reserved_first: [u8; 11],
    attributes: Attributes,
    _reserved_second: [u8; 20],
}

const_assert_size!(VFatUnknownDirEntry, 32);

pub union VFatDirEntry {
    unknown: VFatUnknownDirEntry,
    regular: VFatRegularDirEntry,
    long_filename: VFatLfnDirEntry,
}

pub struct EntryIterator<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    entries: Vec<Entry<HANDLE>>,
    curr: usize,
}

impl<HANDLE: VFatHandle> Dir<HANDLE> {
    /// Finds the entry named `name` in `self` and returns it. Comparison is
    /// case-insensitive.
    ///
    /// # Errors
    ///
    /// If no entry with name `name` exists in `self`, an error of `NotFound` is
    /// returned.
    ///
    /// If `name` contains invalid UTF-8 characters, an error of `InvalidInput`
    /// is returned.
    pub fn find<P: AsRef<OsStr>>(&self, name: P) -> io::Result<Entry<HANDLE>> {
        unimplemented!("Dir::find()")
    }
}

impl<HANDLE: VFatHandle> Iterator for EntryIterator<HANDLE> {
    type Item = Entry<HANDLE>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.entries[self.curr].clone();
        self.curr += 1;
        Some(item)
    }
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    type Entry = Entry<HANDLE>;
    type Iter = EntryIterator<HANDLE>;

    #[allow(safe_packed_borrows)]
    fn entries(&self) -> io::Result<Self::Iter> {
        let mut vfat_entries: Vec<VFatDirEntry> = Vec::new();

        self.vfat.lock(|vfat| -> io::Result<()> {
            let mut bytes: Vec<u8> = Vec::new();
            vfat.read_chain(self.cluster, &mut bytes)?;
            vfat_entries = unsafe { bytes.cast() };
            Ok(())
        })?;

        fn trim(bytes: &[u8]) -> Vec<u8> {
            let mut vec: Vec<u8> = Vec::new();
            for &byte in bytes {
                if byte == 0x00 || byte == 0xFF {
                    break;
                }
                vec.push(byte);
            }
            vec
        }

        let mut curr = 0;
        let mut entries: Vec<Entry<HANDLE>> = Vec::new();

        while curr < entries.len() {
            let mut unknown_dir_entry: VFatUnknownDirEntry = unsafe {
                vfat_entries[curr].unknown
            };

            // Compute the long file name if it exists.
            // TODO: long filename entries are not necessarily in order,
            // need to check sequence number
            let mut long_name: Vec<u16> = Vec::new();
            while unknown_dir_entry.attributes.0 & LONG_FILENAME_MARKER == LONG_FILENAME_MARKER {
                let long_filename = unsafe {
                    vfat_entries[curr].long_filename
                };

                let char_sets: [&[u16]; 3] = [
                    &long_filename.name_first,
                    &long_filename.name_second,
                    &long_filename.name_third,
                ];

                'outer: for char_set in char_sets.iter() {
                    for character in char_set.iter() {
                        let character = *character;
                        if character == 0x00 || character == 0xFF {
                            break 'outer;
                        }
                        long_name.push(character);
                    }
                }

                unknown_dir_entry = unsafe { vfat_entries[curr].unknown };
                curr += 1;
            }

            curr += 1;

            let regular = unsafe {
                vfat_entries[curr].regular
            };


            let short_name = trim(&regular.name);
            let extension = trim(&regular.extension);

            let name = String::from_utf16(&long_name.as_slice()).unwrap() +
                from_utf8(short_name.as_slice()).unwrap() +
                "." + from_utf8(extension.as_slice()).unwrap();

            let entry_cluster = (regular.first_cluster_high_16 as u32) << 16 + regular.first_cluster_low_16;

            let is_directory = regular.attributes.0 & 0x10 != 0;
            entries.push(if is_directory {
                // TODO: other fields ie. date created, date modified
                Entry::Dir(
                    Dir {
                        vfat: self.vfat.clone(),
                        cluster: Cluster { 0: entry_cluster },
                        name
                    }
                )
            } else {
                Entry::File(
                    File {
                        vfat: self.vfat.clone(),
                    }
                )
            });
        }

        Ok(EntryIterator{
            phantom: PhantomData,
            curr: 0,
            entries,
        })
    }
}
