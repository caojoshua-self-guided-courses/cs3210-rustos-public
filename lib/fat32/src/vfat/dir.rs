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
const LONG_FILENAME_MAX_CHARS: u8 = 13;

#[derive(Clone, Debug)]
pub struct Dir<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub cluster: Cluster,
    pub name: String,
    pub metadata: Metadata,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct VFatRegularDirEntry {
    name: [u8; 8],
    extension: [u8; 3],
    pub attributes: Attributes,
    windows_nt_reserved: u8,
    pub creation_time_tenth_seconds: u8,
    pub create_timestamp: Timestamp,
    pub last_accessed_date: Date,
    pub first_cluster_high_16: u16,
    pub last_modification_timestamp: Timestamp,
    pub first_cluster_low_16: u16,
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
    first_byte: u8,
    _reserved_first: [u8; 10],
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
        let name = match name.as_ref().to_str() {
            Some(str) => str,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "`name` contains invalid UTF-8 characters",
                ))
            }
        };

        for entry in traits::Dir::entries(self)? {
            if traits::Entry::name(&entry) == name {
                return Ok(entry);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("`{}` not found in `{}`", name, self.name),
        ))
    }
}

impl<HANDLE: VFatHandle> Iterator for EntryIterator<HANDLE> {
    type Item = Entry<HANDLE>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr >= self.entries.len() {
            None
        } else {
            let item = self.entries[self.curr].clone();
            self.curr += 1;
            Some(item)
        }
    }
}

impl<HANDLE: VFatHandle> traits::Dir for Dir<HANDLE> {
    type Entry = Entry<HANDLE>;
    type Iter = EntryIterator<HANDLE>;

    #[allow(safe_packed_borrows)]
    fn entries(&self) -> io::Result<Self::Iter> {
        println!("getting entries for {}", self.name);
        let vfat_entries = self.vfat.lock(|vfat| -> io::Result<Vec<VFatDirEntry>> {
            let mut bytes: Vec<u8> = Vec::new();
            vfat.read_chain(self.cluster, &mut bytes)?;
            Ok(unsafe { bytes.cast() })
        })?;

        let mut curr = 0;
        let mut entries: Vec<Entry<HANDLE>> = Vec::new();

        'outer: while curr < vfat_entries.len() {
            let mut unknown_dir_entry: VFatUnknownDirEntry = unsafe { vfat_entries[curr].unknown };

            // Ignore entry if it is deleted/unused.
            if unknown_dir_entry.first_byte == 0xE5 {
                curr += 1;
                continue;
            }

            // Compute the long file name if it exists.
            let mut long_name: Vec<u16> = Vec::new();
            while unknown_dir_entry.attributes.0 & LONG_FILENAME_MARKER == LONG_FILENAME_MARKER {
                let long_filename = unsafe { vfat_entries[curr].long_filename };

                // Compute the index for this long file name entry
                let lfn_sequence_number = (long_filename.sequence_number & 0b1111) - 1;
                let mut lfn_idx = lfn_sequence_number * LONG_FILENAME_MAX_CHARS;

                // Resize long_name if the new sequnce number is the largest seen so far.
                let diff = (lfn_idx + LONG_FILENAME_MAX_CHARS) as i32 - long_name.len() as i32;
                if diff > 0 {
                    long_name.resize(long_name.len() + diff as usize, 0);
                }

                let char_sets: [&[u16]; 3] = [
                    &long_filename.name_first,
                    &long_filename.name_second,
                    &long_filename.name_third,
                ];

                // Insert character into long_filename
                for char_set in char_sets.iter() {
                    for character in char_set.iter() {
                        long_name[lfn_idx as usize] = *character;
                        lfn_idx += 1;
                    }
                }

                curr += 1;
                if curr >= vfat_entries.len() {
                    break 'outer;
                }
                unknown_dir_entry = unsafe { vfat_entries[curr].unknown };
            }

            let regular = unsafe { vfat_entries[curr].regular };
            // If the first character in the name is 0x00, the previous entry is the last entry.
            if regular.name[0] == 0x00 {
                break;
            }
            curr += 1;

            // Helper to return a Vector with characters up until
            // terminating characters
            fn trim<T: Copy + Into<u16>>(bytes: &[T]) -> Vec<T> {
                let mut vec: Vec<T> = Vec::new();
                for &byte in bytes {
                    if byte.into() == 0x00 || byte.into() == 0x20 {
                        break;
                    }
                    vec.push(byte);
                }
                vec
            }

            let long_name_original_len = long_name.len();
            let mut name = String::from_utf16(&trim(long_name.as_slice())).unwrap();

            // If the long name was trimmed, then we should not use the regular
            // file name.
            if name.len() == long_name_original_len {
                name += from_utf8(trim(&regular.name).as_slice()).unwrap();
            }

            // Add the file extension to the filename if its lenght is >0
            let extension = trim(&regular.extension);
            if extension.len() > 0 {
                name += ".";
                name += from_utf8(extension.as_slice()).unwrap();
            }
            println!("{}", name);

            let entry_cluster =
                ((regular.first_cluster_high_16 as u32) << 16) + regular.first_cluster_low_16 as u32;

            let is_directory = regular.attributes.0 & 0x10 != 0;
            entries.push(if is_directory {
                // TODO: other fields ie. date created, date modified
                Entry::Dir(Dir {
                    vfat: self.vfat.clone(),
                    cluster: Cluster { 0: entry_cluster },
                    name,
                    metadata: Metadata::from(regular),
                })
            } else {
                Entry::File(File {
                    vfat: self.vfat.clone(),
                    cluster: Cluster { 0: entry_cluster },
                    name,
                    size: regular.size as u64,
                    seek_pos: 0,
                    metadata: Metadata::from(regular),
                })
            });
        }

        println!("there are {} entries", entries.len());
        Ok(EntryIterator {
            phantom: PhantomData,
            curr: 0,
            entries,
        })
    }
}
