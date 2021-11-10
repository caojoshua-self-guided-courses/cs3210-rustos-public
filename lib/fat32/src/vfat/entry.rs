use crate::traits;
use crate::vfat::{Dir, File, Metadata, VFatHandle};
use core::fmt;

// You can change this definition if you want
#[derive(Clone, Debug)]
pub enum Entry<HANDLE: VFatHandle> {
    File(File<HANDLE>),
    Dir(Dir<HANDLE>),
}

// TODO: Implement any useful helper methods on `Entry`.

impl<HANDLE: VFatHandle> traits::Entry for Entry<HANDLE> {
    type File = File<HANDLE>;
    type Dir = Dir<HANDLE>;
    type Metadata = Metadata;

    fn name(&self) -> &str {
        unimplemented!("file::name")
    }

    fn metadata(&self) -> &Self::Metadata {
        unimplemented!("file::name")
    }

    fn as_file(&self) -> Option<&File<HANDLE>> {
        unimplemented!("file::name")
    }

    fn as_dir(&self) -> Option<&Dir<HANDLE>> {
        unimplemented!("file::name")
    }

    fn into_file(self) -> Option<File<HANDLE>> {
        unimplemented!("file::name")
    }

    fn into_dir(self) -> Option<Dir<HANDLE>> {
        unimplemented!("file::name")
    }
}
