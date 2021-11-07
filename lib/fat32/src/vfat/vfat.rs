use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::Path;

use crate::mbr::MasterBootRecord;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::SliceExt;
use crate::vfat::{BiosParameterBlock, CachedPartition, Partition};
use crate::vfat::{Cluster, Dir, Entry, Error, FatEntry, File, Status};

/// A generic trait that handles a critical section as a closure
pub trait VFatHandle: Clone + Debug + Send + Sync {
    fn new(val: VFat<Self>) -> Self;
    fn lock<R>(&self, f: impl FnOnce(&mut VFat<Self>) -> R) -> R;
}

#[derive(Debug)]
pub struct VFat<HANDLE: VFatHandle> {
    phantom: PhantomData<HANDLE>,
    device: CachedPartition,
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    sectors_per_fat: u32,
    fat_start_sector: u64,
    data_start_sector: u64,
    rootdir_cluster: Cluster,
}

impl<HANDLE: VFatHandle> VFat<HANDLE> {
    pub fn from<T>(mut device: T) -> Result<HANDLE, Error>
    where
        T: BlockDevice + 'static,
    {
        let mbr = MasterBootRecord::from(&mut device)?;

        let mut fat_partition_entry = None;
        for partition_entry in &mbr.partition_table {
            if partition_entry.partition_type == 0xB ||
                partition_entry.partition_type == 0xC {
                fat_partition_entry = Some(partition_entry);
            }
        }

        if fat_partition_entry.is_none() {
            return Err(Error::NotFound);
        }
        let fat_partition_entry = fat_partition_entry.unwrap();

        let ebpb = BiosParameterBlock::from(&mut device, fat_partition_entry.relative_sector as u64)?;

        let partition = Partition {
            start: fat_partition_entry.relative_sector as u64,
            num_sectors: fat_partition_entry.total_sectors as u64,
            sector_size: ebpb.bytes_per_sector as u64,
        };
        let cached_partition = CachedPartition::new(device, partition);

        let vfat = VFat {
            phantom: PhantomData,
            device: cached_partition,
            bytes_per_sector: 0,
            sectors_per_cluster: 0,
            sectors_per_fat: 0,
            fat_start_sector: 0,
            data_start_sector: 0,
            rootdir_cluster: Cluster::from(0),
        };
        Ok(VFatHandle::new(vfat))
    }

    //  * A method to read from an offset of a cluster into a buffer.
    //  TODO: I'm supposed to use fat_entry
    fn read_cluster(
        &mut self,
        cluster: Cluster,
        offset: usize,
        buf: &mut Vec<u8>
    ) -> io::Result<usize> {
        if offset >= (self.bytes_per_sector * (self.sectors_per_cluster as u16)) as usize {
            return Ok(0);
        }

        for i in 0..self.sectors_per_cluster {
            // let slice = &mut buf[(i as u16 * self.bytes_per_sector) as usize..];
            let cluster_sector_number = self.data_start_sector + ((cluster.0 + i as u32) as u64);
            self.device.read_all_sector(cluster_sector_number, buf)?;
        }

        Ok(buf.len())
    }

    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
   fn read_chain(
       &mut self,
       start: Cluster,
       buf: &mut Vec<u8>
   ) -> io::Result<usize> {
        let mut cluster = start;
        loop {
            // Need helper to get raw cluster sector number
            self.read_cluster(cluster, 0, buf);

            let fat_entry = match self.fat_entry(cluster) {
                Ok(fat_entry) => fat_entry,
                Err(_) => return Ok(buf.len()),
            };

            // TODO: should we return error if fat_entry status is
            // neither Data or Eoc?
            cluster = match fat_entry.status() {
                Status::Data(cluster) => cluster,
                _ => return Ok(buf.len()),
            };
        };
   }

    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    // fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
    fn fat_entry(&mut self, cluster: Cluster) -> io::Result<FatEntry> {
        Ok(FatEntry{0: self.fat_start_sector as u32 + cluster.0})
    }
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = crate::traits::Dummy;
    type Dir = crate::traits::Dummy;
    type Entry = crate::traits::Dummy;

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        unimplemented!("FileSystem::open()")
    }
}
