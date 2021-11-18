use core::fmt;
use core::mem::size_of;
use shim::const_assert_size;

use crate::traits::BlockDevice;
use crate::vfat::Error;

#[repr(C, packed)]
pub struct BiosParameterBlock {
    eb_xx_90: [u8; 3],
    oem_identifier: u64,
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub num_reserved_sectors: u16,
    pub num_fats: u8,
    max_directory_entries: u16,
    num_logical_sectors: u16,
    media_descriptor_type: u8,
    sectors_per_fat: u16,
    sectors_per_track: u16,
    num_heads_or_sides: u16,
    num_hidden_sectors: u32,
    greater_num_logical_sectors: u32,
    pub sectors_per_fat_32_bit: u32,
    flags: u16,
    fat_minor_version: u8,
    fat_major_version: u8,
    pub root_cluster_num: u32,
    fsinfo_sector_num: u16,
    backup_boot_sector_num: u16,
    reserved: [u8; 12],
    drive_num: u8,
    windows_nt_flags: u8,
    signature: u8,
    volume_id_serial_num: u32,
    volume_label_string: [u8; 11],
    system_id_string: [u8; 8],
    bootcode: [u8; 420],
    bootable_partition_signature: [u8; 2],
}

const_assert_size!(BiosParameterBlock, 512);

impl BiosParameterBlock {
    /// Reads the FAT32 extended BIOS parameter block from sector `sector` of
    /// device `device`.
    ///
    /// # Errors
    ///
    /// If the EBPB signature is invalid, returns an error of `BadSignature`.
    pub fn from<T: BlockDevice>(mut device: T, sector: u64) -> Result<BiosParameterBlock, Error> {
        let mut ebpb_sector: [u8; size_of::<BiosParameterBlock>()] = [0; size_of::<BiosParameterBlock>()];
        match device.read_sector(sector, &mut ebpb_sector) {
            Ok(_) => (),
            Err(error) => return Err(Error::Io(error)),
        };

        let ebpb = unsafe {
            core::mem::transmute::<[u8; size_of::<BiosParameterBlock>()], BiosParameterBlock>(ebpb_sector)
        };

        if ebpb.bootable_partition_signature[0] != 0x55 ||
            ebpb.bootable_partition_signature[1] != 0xAA {
            return Err(Error::BadSignature);
        }

        Ok(ebpb)
    }

    pub fn sectors_per_fat(&self) -> u32 {
        if self.sectors_per_fat == 0 {
            self.sectors_per_fat_32_bit
        } else {
            self.sectors_per_fat.into()
        }

    }
}

impl fmt::Debug for BiosParameterBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("")
            .field("eb_xx_90", &self.eb_xx_90)
            .field("oem_identifier", &{ self.oem_identifier })
            .field("bytes_per_sector", &{ self.bytes_per_sector })
            .field("sectors_per_cluster", &self.sectors_per_cluster)
            .field("num_reserved_sectors", &{ self.num_reserved_sectors })
            .field("num_fats", &self.num_fats)
            .field("max_directory_entires", &{ self.max_directory_entries })
            .field("num_logical_sectors", &{ self.num_logical_sectors })
            .field("media_descriptor_type", &self.media_descriptor_type)
            .field("sectors_per_fat", &{ self.sectors_per_fat })
            .field("sectors_per_track", &{ self.sectors_per_track })
            .field("num_heads_or_sides", &{ self.num_heads_or_sides })
            .field("num_hidden_sectors", &{ self.num_hidden_sectors })
            .field("greater_num_logical_sectors", &{ self.greater_num_logical_sectors })
            .field("sectors_per_fat_32_bit", &{ self.sectors_per_fat_32_bit })
            .field("flags", &{ self.flags })
            .field("fat_minor_version", &self.fat_minor_version)
            .field("fat_major_version", &self.fat_major_version)
            .field("root_cluster_num", &{ self.root_cluster_num })
            .field("fsinfo_sector_num", &{ self.fsinfo_sector_num })
            .field("backup_boot_sector_num", &{ self.backup_boot_sector_num })
            .field("reserved", &self.reserved)
            .field("drive_num", &self.drive_num)
            .field("windows_nt_flags", &self.windows_nt_flags)
            .field("signature", &self.signature)
            .field("volume_id_serial_num", &{ self.volume_id_serial_num })
            .field("volume_label_string", &self.volume_label_string)
            .field("system_id_string", &self.system_id_string)
            .field("bootcode", &"<bootcode>")
            .field("bootcode", &"<bootcode>")
            .field("bootable_partition_signature", &self.bootable_partition_signature)
            .finish()
    }
}
