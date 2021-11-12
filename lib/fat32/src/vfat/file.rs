use alloc::string::String;

use shim::io::{self, SeekFrom, Write};

use crate::traits;
use crate::vfat::{Cluster, Metadata, VFatHandle};

#[derive(Clone, Debug)]
pub struct File<HANDLE: VFatHandle> {
    pub vfat: HANDLE,
    pub cluster: Cluster,
    pub name: String,
    pub size: u64,
    pub seek_pos: u64,
    pub metadata: Metadata,
}

// FIXME: Implement `traits::File` (and its supertraits) for `File`.
impl<HANDLE: VFatHandle> traits::File for File<HANDLE> {
    fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn size(&self) -> u64 {
        self.size
    }
}

impl<HANDLE: VFatHandle> io::Read for File<HANDLE> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        // TODO: use seek_pos
        let mut vec: Vec<u8> = Vec::new();
        self.vfat
            .lock(|vfat| -> io::Result<usize> { vfat.read_chain(self.cluster, &mut vec) })?;
        buf.write(vec.as_slice())?;
        Ok(buf.len())
    }
}

impl<HANDLE: VFatHandle> io::Write for File<HANDLE> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // TODO: use seek_pos
        self.vfat
            .lock(|vfat| -> io::Result<usize> { vfat.write_chain(self.cluster, buf) })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<HANDLE: VFatHandle> io::Seek for File<HANDLE> {
    /// Seek to offset `pos` in the file.
    ///
    /// A seek to the end of the file is allowed. A seek _beyond_ the end of the
    /// file returns an `InvalidInput` error.
    ///
    /// If the seek operation completes successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with SeekFrom::Start.
    ///
    /// # Errors
    ///
    /// Seeking before the start of a file or beyond the end of the file results
    /// in an `InvalidInput` error.
    fn seek(&mut self, _pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match _pos {
            SeekFrom::Start(offset) => offset as i128,
            SeekFrom::End(offset) => self.size as i128 - 1 + offset as i128,
            SeekFrom::Current(offset) => self.seek_pos as i128 + offset as i128,
        };

        if new_pos < 0 {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Seek before 0 in file `{}`", self.name),
            ))
        } else if new_pos > (self.size - 1) as i128 {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Seek beyond end of file `{}` at index `{}`, when size is {}",
                    self.name, new_pos, self.size as i128
                ),
            ))
        } else {
            self.seek_pos = new_pos as u64;
            Ok(self.seek_pos)
        }
    }
}
