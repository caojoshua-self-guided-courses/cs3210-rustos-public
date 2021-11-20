use core::fmt::Debug;
use core::marker::PhantomData;
use core::mem::size_of;

use alloc::vec::Vec;

use shim::io;
use shim::ioerr;
use shim::newioerr;
use shim::path;
use shim::path::{Component, Path};

use crate::mbr::MasterBootRecord;
use crate::traits;
use crate::traits::{BlockDevice, FileSystem};
use crate::util::{SliceExt, VecExt};
use crate::vfat::{BiosParameterBlock, CachedPartition, Metadata, Partition};
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
    root_dir_cluster: Cluster,
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

        let fat_start_sector = ebpb.num_reserved_sectors;
        let data_start_sector = fat_start_sector as u32 + (ebpb.num_fats as u32 * ebpb.sectors_per_fat());

        let vfat = VFat {
            phantom: PhantomData,
            device: cached_partition,
            bytes_per_sector: ebpb.bytes_per_sector,
            sectors_per_cluster: ebpb.sectors_per_cluster,
            sectors_per_fat: ebpb.sectors_per_fat_32_bit,
            fat_start_sector: fat_start_sector.into(),
            data_start_sector: data_start_sector.into(),
            root_dir_cluster: Cluster::from(ebpb.root_cluster_num),
        };
        Ok(VFatHandle::new(vfat))
    }

    //  * A method to read from an offset of a cluster into a buffer.
    //  TODO: deal with offsets
    //  TODO: make code cleaner with helpers for common logic
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
            self.device.read_all_sector(
                self.cluster_raw_sector(Cluster {
                    0: cluster.0 + i as u32,
                }),
                buf,
            )?;
        }

        Ok(buf.len())
    }

    //  * A method to write from a buffer into a cluster from an offset
    fn write_cluster(&mut self, cluster: Cluster, offset: usize, buf: &[u8]) -> io::Result<usize> {
        if offset >= (self.bytes_per_sector * (self.sectors_per_cluster as u16)) as usize {
            return Ok(0);
        }

        let mut bytes_written = 0;
        for i in 0..self.sectors_per_cluster {
            bytes_written += self.device.write_sector(
                self.cluster_raw_sector(Cluster {
                    0: cluster.0 + i as u32,
                }),
                buf,
            )?;
        }

        Ok(bytes_written)
    }
    //  * A method to read all of the clusters chained from a starting cluster
    //    into a vector.
    pub fn read_chain(&mut self, start: Cluster, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut cluster = start;
        loop {
            self.read_cluster(cluster, 0, buf)?;
            let fat_entry = self.fat_entry(cluster)?;

            cluster = match fat_entry.status() {
                Status::Data(cluster) => cluster,
                _ => return Ok(buf.len()),
            };
        }
    }

    //  * A method to write all the contents of buf into all of the clusters chained
    //  from a starting cluster.
    pub fn write_chain(&mut self, start: Cluster, buf: &[u8]) -> io::Result<usize> {
        let mut cluster = start;
        let mut bytes_written = 0;
        loop {
            bytes_written += self.write_cluster(cluster, 0, buf)?;
            let fat_entry = self.fat_entry(cluster)?;

            cluster = match fat_entry.status() {
                Status::Data(cluster) => cluster,
                _ => return Ok(bytes_written),
            };
        }
    }

    //  * A method to return a reference to a `FatEntry` for a cluster where the
    //    reference points directly into a cached sector.
    pub fn fat_entry(&mut self, cluster: Cluster) -> io::Result<FatEntry> {
        let bytes_offset = cluster.0 * size_of::<FatEntry>() as u32;
        let sector = self.fat_start_sector + (bytes_offset / self.bytes_per_sector as u32) as u64;
        let offset = bytes_offset % self.bytes_per_sector as u32;

        let mut bytes: Vec<u8> = Vec::new();
        self.device.read_all_sector(sector, &mut bytes)?;

        let mut fat_entry_val: u32 = 0;
        for i in 0..4 {
            fat_entry_val = (fat_entry_val << 8) + bytes[(offset + i) as usize] as u32;
        }

        Ok(FatEntry::from(fat_entry_val))
    }

    pub fn cluster_raw_sector(&self, cluster: Cluster) -> u64 {
        // data sector starts with cluster 2. fats and sectors are 1-indexed
        // TODO: ???
        // self.data_start_sector + cluster.0 as u64 - 1
        self.data_start_sector + cluster.0 as u64 - 2
    }

    pub fn bytes_per_cluster(&self) -> usize {
        (self.sectors_per_cluster as u16 * self.bytes_per_sector) as usize
    }
}

impl<'a, HANDLE: VFatHandle> FileSystem for &'a HANDLE {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Entry = Entry<HANDLE>;

    fn open_root_dir(self) -> Entry<HANDLE> {
        self.lock(|vfat| {
            Entry::Dir(Dir {
                vfat: self.clone(),
                cluster: Cluster::from(vfat.root_dir_cluster),
                name: String::from(""),
                metadata: Metadata::empty(),
            })
        })
    }

    fn open<P: AsRef<Path>>(self, path: P) -> io::Result<Self::Entry> {
        let mut entry = self.open_root_dir();

        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Path must be absolute",
            ))
        }

        for component in path.components() {
            entry = match component {
                Component::RootDir => self.open_root_dir(),
                Component::CurDir => continue,
                // TODO: support parent directories
                Component::ParentDir => self.open_root_dir(),
                Component::Normal(name) => {
                    let dir_entry = match traits::Entry::as_dir(&entry) {
                        Some(dir_entry) => dir_entry,
                        None => return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("`{}` is not a directory", traits::Entry::name(&entry)),
                        ))
                    };
                    dir_entry.find(name)?
                },
                Component::Prefix(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "RustOS does not accept Windows path prefix in path",
                    ))
                }
            };
        }

        Ok(entry)
    }
}
