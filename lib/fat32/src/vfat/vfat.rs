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
use crate::util::{SliceExt, VecExt};
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
            if partition_entry.partition_type == 0xB || partition_entry.partition_type == 0xC {
                fat_partition_entry = Some(partition_entry);
            }
        }

        if fat_partition_entry.is_none() {
            return Err(Error::NotFound);
        }
        let fat_partition_entry = fat_partition_entry.unwrap();

        let ebpb =
            BiosParameterBlock::from(&mut device, fat_partition_entry.relative_sector as u64)?;

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
        buf: &mut Vec<u8>,
    ) -> io::Result<usize> {
        if offset >= (self.bytes_per_sector * (self.sectors_per_cluster as u16)) as usize {
            return Ok(0);
        }

        for i in 0..self.sectors_per_cluster {
            self.device
                .read_all_sector(self.cluster_raw_sector(Cluster{ 0: cluster.0 + i as u32 }), buf)?;
        }

        Ok(buf.len())
    }

    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    pub fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut cluster = start;
        loop {
            self.read_cluster(cluster, 0, buf)?;
            let fat_entry = self.fat_entry(cluster)?;

            // TODO: should we return error if fat_entry status is
            // neither Data or Eoc?
            cluster = match fat_entry.status() {
                Status::Data(cluster) => cluster,
                _ => return Ok(buf.len()),
            };
        }
    }

    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    // fn fat_entry(&mut self, cluster: Cluster) -> io::Result<&FatEntry> {
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<FatEntry> {
        let bytes_offset = cluster.0 * size_of::<FatEntry>() as u32;
        let sector = self.fat_start_sector + (bytes_offset / self.bytes_per_sector as u32) as u64;
        let offset = bytes_offset % self.bytes_per_sector as u32;

        let mut bytes: Vec<u8> = Vec::new();
        self.device.read_all_sector(sector, &mut bytes)?;
        let bytes: Vec<u32> = unsafe { bytes.cast() };

        Ok(FatEntry { 0: bytes[offset as usize] })
    }

    // pub fn cluster(&mut self, fat_entry: FatEntry) -> Cluster {
    //     Cluster {0: self.data_start_sector + fat_entry.0 }

    // }

    pub fn cluster_raw_sector(&self, cluster: Cluster) -> u64 {
        // data sector starts with cluster 2
        self.data_start_sector + cluster.0 as u64 - 2
    }

    pub fn bytes_per_cluster(&self) -> usize {
        (self.sectors_per_cluster as u16 * self.bytes_per_sector) as usize
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
