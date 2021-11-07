use core::fmt;
use core::mem::size_of;
use shim::const_assert_size;
use shim::io;

use crate::traits::BlockDevice;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CHS {
    head: u8,
    sector_and_cylinder: [u8; 2],
}

impl fmt::Debug for CHS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CachedPartition")
            .field("head", &self.head)
            .field("sector_and_cylinder", &self.sector_and_cylinder)
            .finish()
    }
}

const_assert_size!(CHS, 3);

#[repr(C, packed)]
pub struct PartitionEntry {
    boot_indicator: u8,
    starting_chs: CHS,
    pub partition_type: u8,
    ending_chs: CHS,
    pub relative_sector: u32,
    pub total_sectors: u32,
}

impl fmt::Debug for PartitionEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PartitionEntry")
            .field("boot_indicator", &self.boot_indicator)
            .field("starting_chs", &self.starting_chs)
            .field("partition_type", &self.partition_type)
            .field("ending_chs", &self.ending_chs)
            .field("relative_sector", &{ self.relative_sector })
            .field("total_sectors", &{ self.total_sectors })
            .finish()
    }
}

const_assert_size!(PartitionEntry, 16);

/// The master boot record (MBR).
#[repr(C, packed)]
pub struct MasterBootRecord {
    bootstrap: [u8; 436],
    disk_id: [u8; 10],
    pub partition_table: [PartitionEntry; 4],
    magic_number: [u8; 2],
}

impl fmt::Debug for MasterBootRecord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MasterBootRecord")
            .field("bootstrap", &"<bootstrap>")
            .field("disk_id", &self.disk_id)
            .field("partition_table", &self.partition_table)
            .field("magic_number", &self.magic_number)
            .finish()
    }
}

const_assert_size!(MasterBootRecord, 512);

#[derive(Debug)]
pub enum Error {
    /// There was an I/O error while reading the MBR.
    Io(io::Error),
    /// Partiion `.0` (0-indexed) contains an invalid or unknown boot indicator.
    UnknownBootIndicator(u8),
    /// The MBR magic signature was invalid.
    BadSignature,
}

impl MasterBootRecord {
    /// Reads and returns the master boot record (MBR) from `device`.
    ///
    /// # Errors
    ///
    /// Returns `BadSignature` if the MBR contains an invalid magic signature.
    /// Returns `UnknownBootIndicator(n)` if partition `n` contains an invalid
    /// boot indicator. Returns `Io(err)` if the I/O error `err` occured while
    /// reading the MBR.
    pub fn from<T: BlockDevice>(mut device: T) -> Result<MasterBootRecord, Error> {
        // Read the MBR(0'th) sector to memory.
        let mut mbr_sector: [u8; size_of::<MasterBootRecord>()] = [0; size_of::<MasterBootRecord>()];
        match device.read_sector(0, &mut mbr_sector) {
            Ok(_) => (),
            Err(error) => return Err(Error::Io(error)),
        }

        // Transmute raw sector bytes into a MasterBootRecord struct.
        let mbr = unsafe {
            core::mem::transmute::<[u8; size_of::<MasterBootRecord>()], MasterBootRecord>(mbr_sector)
        };

        // Check that the MBR record has the correct magic number.
        if mbr.magic_number[0] != 0x55 || mbr.magic_number[1] != 0xAA {
            return Err(Error::BadSignature);
        }

        // Check that each partition table entry has a valid boot record.
        for i in 0..mbr.partition_table.len() {
            let partition_entry = &mbr.partition_table[i];
            if partition_entry.boot_indicator != 0
                && partition_entry.boot_indicator != 0x80 {
                return Err(Error::UnknownBootIndicator(i as u8));
            }
        }

        Ok(mbr)
    }
}
