//! Module for paging related issues
//!
//! The pointer to the translation table base can be obtained by
//! ```
//!     TranslationTable::get_current_ttbr0()
//! ```
//! To create new entries in the table, first create a new memory attribute by
//! ```
//!     let attributes = MemoryAttributes::from(ATTRIBUTES::AP::PrivAccess);
//! ```
//! and then create a new section by
//! ```
//!     let section = TableDescriptor::new(TableType::Section, section_physical_address, attributes);
//!     base_table[index] = section;
//! ```
//!
//! A new second level pagetable can be created by
//! ```
//!     let pagetable = unsafe { PageTable::new(pagetable_virtual_address, attributes, base_table, index)? };
//! ```
//! A new entry in the pagetable is created by
//! ```
//!     let small_page = PageDescriptor::new(PageType::SmallPage, physical_address, attributes)?;
//!     pagetable[index_pt] = small_page;
//! ```


use crate::regs::vmem_control::*;
//use crate::regs::ats;
use core::fmt;
use core::ops;
use core::arch::arm;
use crate::{VirtualAddress,PhysicalAddress};
use register::{register_bitfields, FieldValue};

register_bitfields! {
    u32,
    pub ATTRIBUTES [
        PXN OFFSET(0) NUMBITS(1) [Enable = 0b1],
        B OFFSET(2) NUMBITS(1) [Enable = 0b1],
        C OFFSET(3) NUMBITS(1) [Enable = 0b1],
        XN OFFSET(4) NUMBITS(1) [Enable = 0b1],
        DOMAIN OFFSET(5) NUMBITS(4) [],
        AP OFFSET(10) NUMBITS(2) [
            NoAccess = 0b00,
            PrivAccess = 0b01,
            UnprivReadOnly = 0b10,
            FullAccess = 0b11
        ],
        TEX OFFSET(12) NUMBITS(3) [],
        AP2 OFFSET(15) NUMBITS(1) [ReadOnly = 0b1],
        S OFFSET(16) NUMBITS(1) [Enable = 0b1],
        NG OFFSET(17) NUMBITS(1) [Enable = 0b1],
        NS OFFSET(19) NUMBITS(1) [Enable = 0b1]
    ]
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(transparent)]
/// This struct captures all the possible memory attributes of pages
pub struct MemoryAttributes(u32);
// The internal structure is as follows:
// 0---1---2---3---
// PXN-res-B---C---
//
// 4---5---6---7---
// XN--Domain------
//
// 8---9---10--11--
// ----res-AP------
//
// 12--13--14--15--
// TEX---------AP2-
//
// 16--17--18--19--
// S---nG--res-NS--
//
// all higher bits are reserved (read/write 0)
//

impl MemoryAttributes {
    pub fn from_table_descriptor(table_descriptor: TableDescriptor) -> Option<Self> {
        let table_type = table_descriptor.get_type();
        let val = table_descriptor.0;
        let bitset = match table_type {
            TableType::Invalid => return None,
            TableType::Page => {
                // first we copy the domain bits to our output variable
                let mut out = val & 0x01e0;
                // Copy PXN
                out |= (val >> 2) & 1;
                // Copy NS
                out |= (val & 0x8) << 16;
                out
            },
            TableType::Section => {
                // this is easy, since we interally store the memory attributes as in the section
                // table descriptor
                val & 0x000b_fdfd
            },
            TableType::Supersection => {
                // similar to the section, but we have to set the domain (bits 5 to 8) to zero
                val & 0x000d_fa1d
            },
        };
        Some(MemoryAttributes(bitset))
    }
    pub fn from_page_descriptor(page_descriptor: PageDescriptor) -> Self {
        let page_type = page_descriptor.get_type();
        let val = page_descriptor.0;
        match page_type {
            PageType::Invalid => MemoryAttributes(0),
            PageType::LargePage => {
                // first we copy the C, B, and TEX bits to our output variable
                let mut out = val & 0x700c;
                // Copy AP
                out |= (val & 0x0030) << (10 - 4);
                // Copy AP2, S, and nG
                out |= (val & 0x0e00) << (15 - 9);
                // Copy XN
                out |= (val & 0x8000) >> (15 - 4);
                MemoryAttributes(out)
            },
            PageType::SmallPage => {
                // first we copy the C and B bits to our output variable
                let mut out = val & 0b1100;
                // Copy XN
                out |= (val & 0b1) << 4;
                // Copy AP, TEX, AP2, S, and nG
                out |= (val & 0xff0) << (10 - 4);
                MemoryAttributes(out)
            },
        }
    }
    // For the cleaniness of the code, we have these functions here. They are only called in the
    // constructors for the respective descriptors, but the descriptors should not know about the
    // internal implementation of the memory attributes.
    // The functions are not visible, because there is not use-case except for creating
    // descriptors.
    fn to_page_descriptor(self, pagetype: PageType) -> PageDescriptor {
        match pagetype {
            PageType::Invalid => PageDescriptor(0),
            PageType::SmallPage => {
                let mut val = 0b10 | (self.0 & 0b1100);
                // Copy XN
                val |= (self.0 & 0x10) >> 4;
                // Copy AP, TEX, AP2, S, and nG
                val |= (self.0 & 0x3fc00) >> (10 - 4);
                PageDescriptor(val)
            },
            PageType::LargePage => {
                let mut val = 0b1 | (self.0 & 0b1100);
                // Copy AP
                val |= (self.0 & 0xc00) >> (10 - 4);
                // Copy AP2, S, and nG
                val |= (self.0 & 0x38000) >> (15 - 9);
                // Copy XN
                val |= (self.0 & 0x10) << (15 - 4);
                PageDescriptor(val)
            },
        }
    }
    fn to_table_descriptor(self, tabletype: TableType) -> TableDescriptor {
        match tabletype {
            TableType::Invalid => TableDescriptor(0),
            TableType::Page => {
                let mut val = 1 | (self.0 & 0x1e0);
                val |= (self.0 & 1) << 2;
                TableDescriptor(val)
            },
            TableType::Section => TableDescriptor(self.0 | 0x2),
            TableType::Supersection => {
                let val = 0x4_0002 | (self.0 & 0xf_fc1f);
                TableDescriptor(val)
            },
        }
    }
}

impl From<FieldValue<u32, ATTRIBUTES::Register>> for MemoryAttributes {
    fn from(attributes: FieldValue<u32, ATTRIBUTES::Register>) -> Self {
        let attributes_u32 = u32::from(attributes);
        MemoryAttributes(attributes_u32)
    }
}

impl Default for MemoryAttributes {
    fn default() -> Self {
        MemoryAttributes(0)
    }
}

#[derive(Debug,Copy,Clone)]
pub enum PageError {
    AlignError,
    TranslationError,
    DomainError,
    PermissionError,
    InvalidMemory,
    NotInRange,
    IndexError
}

pub type Result<T> = ::core::result::Result<T,PageError>;



pub trait Alignable {
    fn is_aligned(&self, mask: u32) -> bool;
    fn check_align(&self, mask: u32) -> Result<()> {
        if self.is_aligned(mask) { Ok(()) }
        else { Err(PageError::AlignError) }
    }
    fn align(&mut self, mask: u32);
}
impl Alignable for VirtualAddress {
    fn is_aligned(&self, mask: u32) -> bool {
        (self.0 & mask) == 0
    }
    fn align(&mut self, mask: u32) {
        self.0 |= !mask;
    }
}
impl Alignable for PhysicalAddress {
    fn is_aligned(&self, mask: u32) -> bool {
        (self.0 & mask) == 0
    }
    fn align(&mut self, mask: u32) {
        self.0 |= !mask;
    }
}



unsafe fn get_phys_frame(virt_addr: VirtualAddress, privileged: bool, writable: bool) -> u32 {
    let output;
    match (privileged, writable) {
        (true, false) => asm!("mcr p15, 0, $0, c7, c8, 0" :: "r"(virt_addr.as_u32()) :: "volatile"),
        (true, true) => asm!("mcr p15, 0, $0, c7, c8, 1" :: "r"(virt_addr.as_u32()) :: "volatile"),
        (false, false) => asm!("mcr p15, 0, $0, c7, c8, 2" :: "r"(virt_addr.as_u32()) :: "volatile"),
        (false, true) => asm!("mcr p15, 0, $0, c7, c8, 3" :: "r"(virt_addr.as_u32()) :: "volatile"),
    }
    asm!("mrc p15, 0, $0, c7, c4, 0" : "=r"(output) ::: "volatile");
    output
}

unsafe fn _get_phys_frame_pr(virt_addr: VirtualAddress) -> u32 {
    get_phys_frame(virt_addr, true, false)
}


/// This function translates a virtual address to a physical address
pub fn get_phys_addr(virt_addr: VirtualAddress) -> Result<PhysicalAddress> {
    let frame_offset: u32 = virt_addr.as_u32() & 0xfff;
    let output = unsafe { get_phys_frame(virt_addr, true, false) & !0xfff };
    if (output & 0b1) == 0 {
        return Ok(PhysicalAddress::new(output | frame_offset))
    }
    // An error has occured.
    Err(PageError::TranslationError)
}


#[derive(Copy,Clone,Debug)]
pub struct OffsetMapping {
    virt_start: VirtualAddress,
    phys_start: PhysicalAddress,
    size: u32,
}

impl OffsetMapping {
    pub fn new(virt_start: VirtualAddress, phys_start: PhysicalAddress, size: u32) -> Self {
        OffsetMapping {
            virt_start,
            phys_start,
            size,
        }
    }
    pub fn virt_addr_in_range(&self, virt_addr: VirtualAddress) -> bool {
        if virt_addr.as_u32() < self.virt_start.as_u32() {
            return false
        }
        if virt_addr.as_u32() > self.virt_start.as_u32() + self.size {
            return false
        }
        true
    }
    pub fn phys_addr_in_range(&self, phys_addr: PhysicalAddress) -> bool {
        if phys_addr.as_u32() < self.phys_start.as_u32() {
            return false
        }
        if phys_addr.as_u32() > self.phys_start.as_u32() + self.size {
            return false
        }
        true
    }

    /// Given a offset mapping calculate the virtual address from the physical address
    pub fn convert_virt_addr(&self, vaddr: VirtualAddress) -> Result<PhysicalAddress> {
        if !self.virt_addr_in_range(vaddr) {
            return Err(PageError::NotInRange)
        }
        let diff = vaddr.as_u32() - self.virt_start.as_u32();
        Ok(PhysicalAddress::new(self.phys_start.as_u32() + diff))
    }

    /// Given a offset mapping calculate the physical address from the virtual address
    pub fn convert_phys_addr(&self, paddr: PhysicalAddress) -> Result<VirtualAddress> {
        if !self.phys_addr_in_range(paddr) {
            return Err(PageError::NotInRange)
        }
        let diff = paddr.as_u32() - self.phys_start.as_u32();
        Ok(VirtualAddress::new(self.virt_start.as_u32() + diff))
    }
}



#[derive(Copy,Clone,Debug,PartialEq)]
pub enum TableType {
    Invalid,
    Page,
    Section,
    Supersection,
}

impl TableType {
    fn align(self) -> u32 {
        match self {
            TableType::Invalid => 0,
            TableType::Page => 0x3ff,
            TableType::Section => 0xf_ffff,
            TableType::Supersection => 0xf_ffff,
        }
    }
}

#[derive(Copy,Clone,Debug)]
#[repr(transparent)]
pub struct TableDescriptor(u32);

impl Alignable for TableDescriptor {
    fn is_aligned(&self, mask: u32) -> bool {
        (self.0 & mask) == 0
    }
    fn align(&mut self, mask: u32) {
        self.0 |= !mask;
    }
}

impl fmt::LowerHex for TableDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val,f)
    }
}

impl ops::BitOr<u32> for TableDescriptor {
    type Output = Self;
    fn bitor(self, rhs: u32) -> Self {
        TableDescriptor(self.0 | rhs)
    }
}
impl ops::BitOrAssign<u32> for TableDescriptor {
    fn bitor_assign(&mut self, rhs: u32) {
        self.0 |= rhs;
    }
}

impl TableDescriptor {
    pub fn get_type(self) -> TableType {
        // starts with
        // 0b00: invalid
        // 0b01: page
        // 0b1x: section or supersection
        // Bit 18 is 0 for section and 1 for supersection
        match self.0 & 0b11 {
            0b00 => TableType::Invalid,
            0b01 => TableType::Page,
            _ => {
                match self.0 & 0x40000 {
                    0b0 => TableType::Section,
                    _ => TableType::Supersection,
                }
            }
        }
    }
    /// Create a new table descriptor
    pub fn new(tabletype: TableType, addr: PhysicalAddress, attributes: MemoryAttributes) -> Result<TableDescriptor> {
        if tabletype == TableType::Invalid {
            return Ok(TableDescriptor(0))
        }
        addr.check_align(tabletype.align())?;
        let mut out = attributes.to_table_descriptor(tabletype);
        out |= addr.0;
        Ok(out)
    }
}

#[derive(Copy,Clone,Debug,PartialEq)]
pub enum PageType {
    Invalid,
    SmallPage,
    LargePage,
}

impl PageType {
   fn align(self) -> u32 {
        match self {
            PageType::Invalid => 0,
            PageType::SmallPage => 0xfff,
            PageType::LargePage => 0xffff,
        }
    }
}

#[derive(Copy,Clone,Debug)]
#[repr(transparent)]
pub struct PageDescriptor(u32);

impl Alignable for PageDescriptor {
    fn is_aligned(&self, mask: u32) -> bool {
        (self.0 & mask) == 0
    }
    fn align(&mut self, mask: u32) {
        self.0 |= !mask;
    }
}

impl fmt::LowerHex for PageDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val,f)
    }
}

impl ops::BitOr<u32> for PageDescriptor {
    type Output = Self;
    fn bitor(self, rhs: u32) -> Self {
        PageDescriptor(self.0 | rhs)
    }
}
impl ops::BitOrAssign<u32> for PageDescriptor {
    fn bitor_assign(&mut self, rhs: u32) {
        self.0 |= rhs;
    }
}


impl PageDescriptor {
    /// Construct a new page descriptor
    pub fn new(pagetype: PageType, addr: PhysicalAddress, attributes: MemoryAttributes) -> Result<PageDescriptor> {
        if pagetype == PageType::Invalid {
            return Ok(PageDescriptor(0))
        }
        addr.check_align(pagetype.align())?;
        let mut out = attributes.to_page_descriptor(pagetype);
        out |= addr.0;
        Ok(out)
    }
    /// Determine the type of the page descriptor
    pub fn get_type(self) -> PageType {
        // starts with
        // 0b00 Invalid
        // 0b01 LargePage
        // 0b1x SmallPage
        match self.0 & 0b11 {
            0b00 => PageType::Invalid,
            0b01 => PageType::LargePage,
            _ => PageType::SmallPage,
        }
    }
    /// Get the physical base address the page is pointing to.
    pub fn get_addr(self) -> Result<PhysicalAddress> {
        let page_type = self.get_type();
        if page_type == PageType::Invalid { return Err(PageError::InvalidMemory) }
        let strip_addr = self.0 & ( !page_type.align() );
        Ok( PhysicalAddress ( strip_addr ) )
    }
}

pub const TRANSLATION_TABLE_SIZE: usize = 4096; // each section is 1MB big; 4GB / 1MB is 4096
pub const PAGE_TABLE_SIZE: usize = 256; // 4Kb * 256 = 1Mb

#[repr(transparent)]
struct TranslationTableMemory {
    table: [TableDescriptor; TRANSLATION_TABLE_SIZE],
}

#[derive(Debug)]
pub struct TranslationTable {
    pointer: *mut TranslationTableMemory,
}

impl TranslationTable {
    /// Create a pointer to a base table from a virtual address
    ///
    /// # Safety
    /// The caller must garantee that the virtual address maps to a base table
    pub unsafe fn new(virt_addr: VirtualAddress) -> Result<Self> {
        virt_addr.check_align(0x3fff)?;
        Ok(TranslationTable { pointer: virt_addr.as_u32() as *mut _ })
    }
    
    /// This functions is deprecated
    pub fn get_current_ttbr0() -> Self {
        let ttbr0 = TTBR0.get() & !0x3fff;
        TranslationTable { pointer: ttbr0 as *mut _ }
    }

    fn ptr(&self) -> *mut TranslationTableMemory {
        self.pointer
    }
    
    /// Returns the physical address of the translation page table
    pub fn get_phys_addr() -> PhysicalAddress {
        PhysicalAddress::new(TTBR0.get() & !0x3fff)
    }

    /// Sets a new translation page table
    ///
    /// # Safety
    /// The caller must garantee that the new translation page table is valid
    pub unsafe fn set_phys_addr(base_addr: PhysicalAddress) -> Result<()> {
        //use crate::regs::ttbr::*;
        base_addr.check_align(0x3fff)?;
        TTBR0.set(base_addr.as_u32());
        arm::__nop();
        arm::__nop();
        arm::__nop();
        Ok(())
    }
}

impl fmt::LowerHex for TranslationTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.pointer as usize as u32;
        fmt::LowerHex::fmt(&val,f)
    }
}

impl ops::Index<usize> for TranslationTable {
    type Output = TableDescriptor;
    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &(*self.ptr()).table[index] }
    }
}

impl ops::IndexMut<usize> for TranslationTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { &mut (*self.ptr()).table[index] }
    }
}

#[repr(transparent)]
struct PageTableMemory {
    table: [PageDescriptor; PAGE_TABLE_SIZE],
}

#[derive(Debug)]
pub struct PageTable {
    pointer: *mut PageTableMemory,
    descriptor: TableDescriptor,
}

impl PageTable {
    /// Creates a page table at a given virtual address
    ///
    /// # Safety
    /// The caller must garantee that the virtual address is mapped to a valid physical address in
    /// RAM.
    pub unsafe fn new(virt_addr: VirtualAddress, mem_attributes: MemoryAttributes, base_table: &mut TranslationTable, index: usize) -> Result<Self> {
        virt_addr.check_align(0x3ff)?;
        let pointer = virt_addr.as_u32() as *mut PageTableMemory;
        let phys_addr = get_phys_addr(virt_addr)?;
        let descriptor = TableDescriptor::new(TableType::Page, phys_addr, mem_attributes)?;
        base_table[index] = descriptor;
        let page_table = PageTable { pointer, descriptor };
        Ok(page_table)
    }
    fn ptr(&self) -> *mut PageTableMemory {
        self.pointer
    }
}

impl ops::Index<usize> for PageTable {
    type Output = PageDescriptor;
    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &(*self.ptr()).table[index] }
    }
}

impl ops::IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { &mut (*self.ptr()).table[index] }
    }
}

