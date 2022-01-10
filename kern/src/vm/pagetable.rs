use core::iter::Chain;
use core::ops::{Deref, DerefMut};
use core::slice::Iter;

use alloc::boxed::Box;
use alloc::fmt;
use core::alloc::{GlobalAlloc, Layout};

use crate::allocator;
use crate::param::*;
use crate::vm::{PhysicalAddr, VirtualAddr};
use crate::ALLOCATOR;

use aarch64::vmsa::*;
use shim::const_assert_size;

#[repr(C)]
pub struct Page([u8; PAGE_SIZE]);
const_assert_size!(Page, PAGE_SIZE);

impl Page {
    pub const SIZE: usize = PAGE_SIZE;
    pub const ALIGN: usize = PAGE_SIZE;

    fn layout() -> Layout {
        unsafe { Layout::from_size_align_unchecked(Self::SIZE, Self::ALIGN) }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L2PageTable {
    pub entries: [RawL2Entry; 8192],
}
const_assert_size!(L2PageTable, PAGE_SIZE);

impl L2PageTable {
    /// Returns a new `L2PageTable`
    fn new() -> L2PageTable {
        L2PageTable{ entries: [RawL2Entry::new(0); 8192] }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(self as *const L2PageTable)
    }
}

#[derive(Copy, Clone)]
pub struct L3Entry(RawL3Entry);

impl L3Entry {
    /// Returns a new `L3Entry`.
    fn new() -> L3Entry {
        L3Entry{ 0: RawL3Entry::new(0) }
    }

    /// Returns `true` if the L3Entry is valid and `false` otherwise.
    fn is_valid(&self) -> bool {
        self.0.get_value(RawL3Entry::VALID) == 1
    }

    /// Extracts `ADDR` field of the L3Entry and returns as a `PhysicalAddr`
    /// if valid. Otherwise, return `None`.
    fn get_page_addr(&self) -> Option<PhysicalAddr> {
        if self.is_valid() {
            Some(PhysicalAddr::from(self.0.get_value(RawL3Entry::ADDR)))
        } else {
            None
        }
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct L3PageTable {
    pub entries: [L3Entry; 8192],
}
const_assert_size!(L3PageTable, PAGE_SIZE);

impl L3PageTable {
    /// Returns a new `L3PageTable`.
    fn new() -> L3PageTable {
        L3PageTable{ entries: [L3Entry::new(); 8192] }
    }

    /// Returns a `PhysicalAddr` of the pagetable.
    pub fn as_ptr(&self) -> PhysicalAddr {
        PhysicalAddr::from(self as *const L3PageTable)
    }
}

#[repr(C)]
#[repr(align(65536))]
pub struct PageTable {
    pub l2: L2PageTable,
    pub l3: [L3PageTable; 2],
}

impl PageTable {
    /// Returns a new `Box` containing `PageTable`.
    /// Entries in L2PageTable should be initialized properly before return.
    fn new(perm: u64) -> Box<PageTable> {
        let mut page_table = Box::new(PageTable{
            l2: L2PageTable::new(),
            l3: [L3PageTable::new(), L3PageTable::new()],
        });

        // Setup L2 page table entries and point them to L3 page tables.
        for i in 0..2 {
            page_table.l2.entries[i].set_value(EntryValid::Valid, RawL3Entry::VALID)
                .set_value(PageType::Page, RawL2Entry::TYPE)
                .set_value(EntryAttr::Mem, RawL2Entry::ATTR)
                .set_value(perm, RawL2Entry::AP)
                .set_value(EntrySh::ISh, RawL2Entry::SH)
                .set_value(1, RawL2Entry::AF)
               // ADDR bits form bits 47:16 of the address of the L3 page table.
               .set_value(&page_table.l3[i] as *const L3PageTable as u64 >> 16, RawL2Entry::ADDR);
        }

        page_table
    }

    /// Returns the (L2index, L3index) extracted from the given virtual address.
    /// Since we are only supporting 1GB virtual memory in this system, L2index
    /// should be smaller than 2.
    ///
    /// # Panics
    ///
    /// Panics if the virtual address is not properly aligned to page size.
    /// Panics if extracted L2index exceeds the number of L3PageTable.
    fn locate(va: VirtualAddr) -> (usize, usize) {
        let va = va.as_usize();
        if va % PAGE_SIZE != 0 {
            panic!("virtual address {} not aligned by page size", va);
        }

        let l2index = va >> 29 & 0b1111111111111;
        if l2index >= 2 {
            panic!("L2index was {}, but there are only 2 L3 page tables", l2index);
        }

        let l3index = va >> 16 & 0b1111111111111;
        (l2index, l3index)
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is valid.
    /// Otherwise, `false` is returned.
    pub fn is_valid(&self, va: VirtualAddr) -> bool {
        let (l2index, l3index) = PageTable::locate(va);
        self.l3[l2index].entries[l3index].is_valid()
    }

    /// Returns `true` if the L3entry indicated by the given virtual address is invalid.
    /// Otherwise, `true` is returned.
    pub fn is_invalid(&self, va: VirtualAddr) -> bool {
        !self.is_valid(va)
    }

    /// Set the given RawL3Entry `entry` to the L3Entry indicated by the given virtual
    /// address.
    pub fn set_entry(&mut self, va: VirtualAddr, entry: RawL3Entry) -> &mut Self {
        let (l2index, l3index) = PageTable::locate(va);
        self.l3[l2index].entries[l3index].0 = entry;
        self
    }

    /// Returns a base address of the pagetable. The returned `PhysicalAddr` value
    /// will point the start address of the L2PageTable.
    pub fn get_baddr(&self) -> PhysicalAddr {
        return self.l2.as_ptr()
    }
}

// FIXME: Implement `IntoIterator` for `&PageTable`.
impl<'a> IntoIterator for &'a PageTable {
    type Item = &'a L3Entry;
    type IntoIter = Chain<Iter<'a, L3Entry>, Iter<'a, L3Entry>>;

    fn into_iter(self) -> Self::IntoIter {
        self.l3[0].entries.iter().chain(self.l3[1].entries.iter())
    }
}

pub struct KernPageTable(Box<PageTable>);

impl KernPageTable {
    /// Returns a new `KernPageTable`. `KernPageTable` should have a `Pagetable`
    /// created with `KERN_RW` permission.
    ///
    /// Set L3entry of ARM physical address starting at 0x00000000 for RAM and
    /// physical address range from `IO_BASE` to `IO_BASE_END` for peripherals.
    /// Each L3 entry should have correct value for lower attributes[10:0] as well
    /// as address[47:16]. Refer to the definition of `RawL3Entry` in `vmsa.rs` for
    /// more details.
    pub fn new() -> KernPageTable {
        let mut page_table = PageTable::new(EntryPerm::KERN_RW);
        let (_, memory_end) = crate::allocator::memory_map().unwrap();

        let mut entry = RawL3Entry::new(0);
        entry.set_value(EntryValid::Valid, RawL3Entry::VALID)
            .set_value(PageType::Page, RawL3Entry::TYPE)
            .set_value(EntryPerm::KERN_RW, RawL3Entry::AP)
            .set_value(1, RawL3Entry::AF);

        // Allocate regular memory.
        let mut addr: usize = 0;
        while addr < memory_end {
            entry.set_value((addr >> 16) as u64, RawL3Entry::ADDR)
                .set_value(EntryAttr::Mem, RawL3Entry::ATTR)
                .set_value(EntrySh::ISh, RawL3Entry::SH);
            page_table.set_entry(VirtualAddr::from(addr), entry);
            addr += PAGE_SIZE;
        }

        // Allocate device memory.
        addr = IO_BASE;
        while addr < IO_BASE_END {
            entry.set_value((addr >> 16) as u64, RawL3Entry::ADDR)
                .set_value(EntryAttr::Dev, RawL3Entry::ATTR)
                .set_value(EntrySh::OSh, RawL3Entry::SH);
            page_table.set_entry(VirtualAddr::from(addr), entry);
            addr += PAGE_SIZE;

        }

        KernPageTable{ 0: page_table }
    }
}

pub enum PagePerm {
    RW,
    RO,
    RWX,
}

pub struct UserPageTable(Box<PageTable>);

impl UserPageTable {
    /// Returns a new `UserPageTable` containing a `PageTable` created with
    /// `USER_RW` permission.
    pub fn new() -> UserPageTable {
        UserPageTable { 0: PageTable::new(EntryPerm::USER_RW) }
    }

    /// Allocates a page and set an L3 entry translates given virtual address to the
    /// physical address of the allocated page. Returns the allocated page.
    ///
    /// # Panics
    /// Panics if the virtual address is lower than `USER_IMG_BASE`.
    /// Panics if the virtual address has already been allocated.
    /// Panics if allocator fails to allocate a page.
    ///
    /// TODO. use Result<T> and make it failurable
    /// TODO. use perm properly
    pub fn alloc(&mut self, va: VirtualAddr, _perm: PagePerm) -> &mut [u8] {
        if va.as_usize() < USER_IMG_BASE {
            panic!("va {} is less than USER_IMG_BASE {}", va.as_usize(), USER_IMG_BASE);
        }

        // Subtract USER_IMG_BASE from va before page table lookup.
        let user_va = va - VirtualAddr::from(USER_IMG_BASE);

        let (l2index, l3index) = PageTable::locate(user_va);
        if self.l3[l2index].entries[l3index].is_valid() {
            panic!("va {} already allocated", va.as_usize());
        }

        // Allocate memory for the new page.
        let addr = unsafe { ALLOCATOR.alloc(Page::layout()) };
        if addr == core::ptr::null_mut() {
            panic!("failed to allocate page for va {}", va.as_usize());
        }

        let perm = match _perm {
            PagePerm::RO => EntryPerm::USER_RO,
            _ => EntryPerm::USER_RW,
        };

        let mut l3entry = L3Entry::new();
        l3entry.0.set_value(EntryValid::Valid, RawL3Entry::VALID)
            .set_value(PageType::Page, RawL3Entry::TYPE)
            .set_value(EntryAttr::Mem, RawL3Entry::ATTR)
            .set_value(perm, RawL3Entry::AP)
            .set_value(EntrySh::ISh, RawL3Entry::SH)
            .set_value(1, RawL3Entry::AF)
            // ADDR field contains bits 47:16 of the memory address.
            .set_value(addr as u64 >> 16, RawL3Entry::ADDR);

        self.l3[l2index].entries[l3index] = l3entry;

        unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, PAGE_SIZE) }
    }
}

impl Deref for KernPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for UserPageTable {
    type Target = PageTable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KernPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DerefMut for UserPageTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for UserPageTable {
    fn drop(&mut self) {
        for entry in self.into_iter() {
            if let Some(mut addr) = entry.get_page_addr() {
                unsafe { ALLOCATOR.dealloc(addr.as_mut_ptr(), Page::layout()) };
            }
        }
    }
}

impl fmt::Debug for UserPageTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}
