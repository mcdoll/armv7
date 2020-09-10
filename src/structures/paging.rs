//! Module for paging related issues
//!
//! To create a new translation table use
//! ```
//!     static mut TTBR0 = TranslationTableMemory::new();
//! ```
//! Create a reference
//! ```
//!     let base_table = TranslationTable::new(&mut TTRB0);
//! ```
//! To create new entries in the table, first create a new memory attribute by
//! ```
//!     let attributes = MemoryAttributes::from(ATTRIBUTES::AP::PrivAccess);
//! ```
//! and then create a new section by
//! ```
//!     let section = TranslationTableDescriptor::new(TranslationTableType::Section, section_physical_address, attributes);
//!     unsafe { base_table.table_mut()[index] = section };
//! ```
//!
//! Finally active the translation table by
//! ```
//!     unsafe { base_table.set_as_ttbr0() };
//! ```
//!
//! A new second level pagetable can be created by
//! ```
//!     let pagetable = unsafe { PageTable::new(&mut page_table_memory, attributes, base_table, index)? };
//! ```
//! A new entry in the pagetable is created by
//! ```
//!     let small_page = PageTableDescriptor::new(PageTableType::SmallPage, physical_address, attributes)?;
//!     unsafe { pagetable.table_mut()[index_pt] = small_page };
//! ```

use crate::regs::vmem_control::*;
use crate::{PhysicalAddress, VirtualAddress};
use core::arch::arm;
use core::fmt;
use core::ops;
use register::{register_bitfields, FieldValue};

register_bitfields! {
    u32,
    ATTRIBUTES [
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
/// This struct contains all the possible memory attributes of various page types
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
    pub fn from_table_descriptor(table_descriptor: TranslationTableDescriptor) -> Option<Self> {
        let table_type = table_descriptor.get_type();
        let val = table_descriptor.0;
        let bitset = match table_type {
            TranslationTableType::Invalid => return None,
            TranslationTableType::Page => {
                // first we copy the domain bits to our output variable
                let mut out = val & 0x01e0;
                // Copy PXN
                out |= (val >> 2) & 1;
                // Copy NS
                out |= (val & 0x8) << 16;
                out
            }
            TranslationTableType::Section => {
                // this is easy, since we interally store the memory attributes as in the section
                // table descriptor
                val & 0x000b_fdfd
            }
            TranslationTableType::Supersection => {
                // similar to the section, but we have to set the domain (bits 5 to 8) to zero
                val & 0x000d_fa1d
            }
        };
        Some(MemoryAttributes(bitset))
    }
    pub fn from_page_descriptor(page_descriptor: PageTableDescriptor) -> Self {
        let page_type = page_descriptor.get_type();
        let val = page_descriptor.0;
        match page_type {
            PageTableType::Invalid => MemoryAttributes(0),
            PageTableType::LargePage => {
                // first we copy the C, B, and TEX bits to our output variable
                let mut out = val & 0x700c;
                // Copy AP
                out |= (val & 0x0030) << (10 - 4);
                // Copy AP2, S, and nG
                out |= (val & 0x0e00) << (15 - 9);
                // Copy XN
                out |= (val & 0x8000) >> (15 - 4);
                MemoryAttributes(out)
            }
            PageTableType::SmallPage => {
                // first we copy the C and B bits to our output variable
                let mut out = val & 0b1100;
                // Copy XN
                out |= (val & 0b1) << 4;
                // Copy AP, TEX, AP2, S, and nG
                out |= (val & 0xff0) << (10 - 4);
                MemoryAttributes(out)
            }
        }
    }
    // For the cleaniness of the code, we have these functions here. They are only called in the
    // constructors for the respective descriptors, but the descriptors should not know about the
    // internal implementation of the memory attributes.
    // The functions are not visible, because there is not use-case except for creating
    // descriptors.
    fn to_page_descriptor(self, pagetype: PageTableType) -> PageTableDescriptor {
        match pagetype {
            PageTableType::Invalid => PageTableDescriptor(0),
            PageTableType::SmallPage => {
                let mut val = 0b10 | (self.0 & 0b1100);
                // Copy XN
                val |= (self.0 & 0x10) >> 4;
                // Copy AP, TEX, AP2, S, and nG
                val |= (self.0 & 0x3fc00) >> (10 - 4);
                PageTableDescriptor(val)
            }
            PageTableType::LargePage => {
                let mut val = 0b1 | (self.0 & 0b1100);
                // Copy AP
                val |= (self.0 & 0xc00) >> (10 - 4);
                // Copy AP2, S, and nG
                val |= (self.0 & 0x38000) >> (15 - 9);
                // Copy XN
                val |= (self.0 & 0x10) << (15 - 4);
                PageTableDescriptor(val)
            }
        }
    }
    fn to_table_descriptor(self, tabletype: TranslationTableType) -> TranslationTableDescriptor {
        match tabletype {
            TranslationTableType::Invalid => TranslationTableDescriptor(0),
            TranslationTableType::Page => {
                let mut val = 1 | (self.0 & 0x1e0);
                val |= (self.0 & 1) << 2;
                TranslationTableDescriptor(val)
            }
            TranslationTableType::Section => TranslationTableDescriptor(self.0 | 0x2),
            TranslationTableType::Supersection => {
                let val = 0x4_0002 | (self.0 & 0xf_fc1f);
                TranslationTableDescriptor(val)
            }
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

#[derive(Debug, Copy, Clone)]
pub enum PageError {
    AlignError,
    TranslationError,
    DomainError,
    PermissionError,
    InvalidMemory,
    NotInRange,
    IndexError,
}

pub type Result<T> = ::core::result::Result<T, PageError>;

trait Alignable {
    fn is_aligned(&self, mask: u32) -> bool;
    fn check_align(&self, mask: u32) -> Result<()> {
        if self.is_aligned(mask) {
            Ok(())
        } else {
            Err(PageError::AlignError)
        }
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

/// Translate MMIO address range to virtual addresses
pub struct DeviceVmemMapper {
    base_address: VirtualAddress,
    device_base_addresses: &'static [u8],
}

impl DeviceVmemMapper {
    /// Create a new mapper
    ///
    /// base_address is the base of the virtual address range that will be used
    /// device_base_addresses is a list of the leading bits of the physical addresses
    pub fn new(base_address: VirtualAddress, device_base_addresses: &'static [u8]) -> Result<Self> {
        base_address.check_align(0x00ff_ffff)?;
        let out = DeviceVmemMapper {
            base_address,
            device_base_addresses,
        };
        Ok(out)
    }

    /// Create a new mapper
    ///
    /// base_address is the base of the virtual address range that will be used
    /// device_base_addresses is a list of the leading bits of the physical addresses
    /// # Safety
    /// This function does not check whether the base address is aligned to 16MB,
    /// ie base_address has to be of the form 0x??00_0000
    pub const unsafe fn new_const(
        base_address: VirtualAddress,
        device_base_addresses: &'static [u8],
    ) -> Self {
        DeviceVmemMapper {
            base_address,
            device_base_addresses,
        }
    }

    /// Perform the mapping
    /// # Safety
    /// This function should only be called once and the virtual address range has to be empty.
    // Actually, calling the function twice does not hurt
    pub unsafe fn do_mapping(&self, base_table: &mut TranslationTable) -> Result<()> {
        let attributes =
            MemoryAttributes::from(ATTRIBUTES::AP::PrivAccess + ATTRIBUTES::XN::Enable);
        let mut base_addr = self.base_address;
        for addr in self.device_base_addresses.iter() {
            let tt_index = base_addr.translation_table_index();
            let device_base = PhysicalAddress::new((*addr as u32) << 24);
            // Each 16MB supersection uses 16 1MB sections
            for index in 0..15 {
                let section = TranslationTableDescriptor::new(
                    TranslationTableType::Section,
                    device_base + 0x10_0000 * index as u32,
                    attributes,
                )?;
                base_table.table_mut()[tt_index + index as usize] = section;
            }
            // increment the base address
            base_addr += 0x0100_0000 as u32;
        }
        Ok(())
    }
    /// Lookup virtual addresses from physical ones
    pub fn lookup(&self, phys_addr: PhysicalAddress) -> Option<VirtualAddress> {
        let phys_index = phys_addr.as_u32() >> 24;
        match self
            .device_base_addresses
            .iter()
            .position(|&y| y == phys_index as u8)
        {
            None => None,
            Some(index) => {
                let naked_phys_addr = phys_addr.as_u32() & 0x00ff_ffff;
                let out = self.base_address + (index << 24);
                Some(out | naked_phys_addr)
            }
        }
    }
}

/// Calculate the physical frame from a given virtual address
///
/// # Safety
/// This will give garbled output if virt_addr is not mapped
/// The user has to check whether the output indicates an error or a valid address
unsafe fn get_phys_frame(virt_addr: VirtualAddress, privileged: bool, writable: bool) -> u32 {
    let output;
    match (privileged, writable) {
        (true, false) => {
            llvm_asm!("mcr p15, 0, $0, c7, c8, 0" :: "r"(virt_addr.as_u32()) :: "volatile")
        }
        (true, true) => {
            llvm_asm!("mcr p15, 0, $0, c7, c8, 1" :: "r"(virt_addr.as_u32()) :: "volatile")
        }
        (false, false) => {
            llvm_asm!("mcr p15, 0, $0, c7, c8, 2" :: "r"(virt_addr.as_u32()) :: "volatile")
        }
        (false, true) => {
            llvm_asm!("mcr p15, 0, $0, c7, c8, 3" :: "r"(virt_addr.as_u32()) :: "volatile")
        }
    }
    llvm_asm!("mrc p15, 0, $0, c7, c4, 0" : "=r"(output) ::: "volatile");
    output
}

unsafe fn _get_phys_frame_pr(virt_addr: VirtualAddress) -> u32 {
    get_phys_frame(virt_addr, true, false)
}

/// Translate a virtual address to a physical address
pub fn get_phys_addr(virt_addr: VirtualAddress) -> Result<PhysicalAddress> {
    let frame_offset: u32 = virt_addr.as_u32() & 0xfff;
    let output = unsafe { get_phys_frame(virt_addr, true, false) & !0xfff };
    if (output & 0b1) == 0 {
        return Ok(PhysicalAddress::new(output | frame_offset));
    }
    // An error has occured.
    Err(PageError::TranslationError)
}

#[derive(Copy, Clone, Debug)]
/// Models an offset mapping between virtual and physical memory
pub struct OffsetMapping {
    virt_start: VirtualAddress,
    phys_start: PhysicalAddress,
    size: u32,
}

impl OffsetMapping {
    /// Create a new offset mapping
    pub const fn new(virt_start: VirtualAddress, phys_start: PhysicalAddress, size: u32) -> Self {
        OffsetMapping {
            virt_start,
            phys_start,
            size,
        }
    }
    /// Checks whether a virtual address is in the range of the offset mapping
    pub fn virt_addr_in_range(&self, virt_addr: VirtualAddress) -> bool {
        if virt_addr.as_u32() < self.virt_start.as_u32() {
            return false;
        }
        if virt_addr.as_u32() > self.virt_start.as_u32() + self.size {
            return false;
        }
        true
    }
    /// Checks whether a physical address is in the range of the offset mapping
    pub fn phys_addr_in_range(&self, phys_addr: PhysicalAddress) -> bool {
        if phys_addr.as_u32() < self.phys_start.as_u32() {
            return false;
        }
        if phys_addr.as_u32() > self.phys_start.as_u32() + self.size {
            return false;
        }
        true
    }

    /// Given a offset mapping calculate the virtual address from the physical address
    pub fn convert_virt_addr(&self, vaddr: VirtualAddress) -> Result<PhysicalAddress> {
        if !self.virt_addr_in_range(vaddr) {
            return Err(PageError::NotInRange);
        }
        let diff = vaddr.as_u32() - self.virt_start.as_u32();
        Ok(PhysicalAddress::new(self.phys_start.as_u32() + diff))
    }

    /// Given a offset mapping calculate the physical address from the virtual address
    pub fn convert_phys_addr(&self, paddr: PhysicalAddress) -> Result<VirtualAddress> {
        if !self.phys_addr_in_range(paddr) {
            return Err(PageError::NotInRange);
        }
        let diff = paddr.as_u32() - self.phys_start.as_u32();
        Ok(VirtualAddress::new(self.virt_start.as_u32() + diff))
    }
}

//
// Translation table stuff
//
//

#[derive(Copy, Clone, Debug, PartialEq)]
/// Different types of entries in a translation table
pub enum TranslationTableType {
    Invalid,
    Page,
    Section,
    Supersection,
}

impl TranslationTableType {
    fn align(self) -> u32 {
        match self {
            TranslationTableType::Invalid => 0,
            TranslationTableType::Page => 0x3ff,
            TranslationTableType::Section => 0xf_ffff,
            TranslationTableType::Supersection => 0xf_ffff,
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
/// A descriptor for a translation table entry
pub struct TranslationTableDescriptor(u32);

impl fmt::LowerHex for TranslationTableDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val, f)
    }
}

impl ops::BitOr<u32> for TranslationTableDescriptor {
    type Output = Self;
    fn bitor(self, rhs: u32) -> Self {
        TranslationTableDescriptor(self.0 | rhs)
    }
}
impl ops::BitOrAssign<u32> for TranslationTableDescriptor {
    fn bitor_assign(&mut self, rhs: u32) {
        self.0 |= rhs;
    }
}

impl TranslationTableDescriptor {
    /// Create a new invalid table descriptor
    pub const fn new_empty() -> Self {
        Self(0)
    }
    /// Create a new table descriptor
    pub fn new(
        tabletype: TranslationTableType,
        addr: PhysicalAddress,
        attributes: MemoryAttributes,
    ) -> Result<Self> {
        if tabletype == TranslationTableType::Invalid {
            return Ok(TranslationTableDescriptor(0));
        }
        addr.check_align(tabletype.align())?;
        let mut out = attributes.to_table_descriptor(tabletype);
        out |= addr.0;
        Ok(out)
    }
    /// Determine the type of the table descriptor
    pub fn get_type(self) -> TranslationTableType {
        // starts with
        // 0b00: invalid
        // 0b01: page
        // 0b1x: section or supersection
        // Bit 18 is 0 for section and 1 for supersection
        match self.0 & 0b11 {
            0b00 => TranslationTableType::Invalid,
            0b01 => TranslationTableType::Page,
            _ => match self.0 & 0x40000 {
                0b0 => TranslationTableType::Section,
                _ => TranslationTableType::Supersection,
            },
        }
    }
}

pub const TRANSLATION_TABLE_SIZE: usize = 4096; // each section is 1MB big; 4GB / 1MB is 4096

#[repr(C)]
#[repr(align(16384))]
/// Base level translation table memory
pub struct TranslationTableMemory {
    table: [TranslationTableDescriptor; TRANSLATION_TABLE_SIZE],
}

impl TranslationTableMemory {
    pub const fn new() -> Self {
        Self {
            table: [TranslationTableDescriptor::new_empty(); TRANSLATION_TABLE_SIZE],
        }
    }
}

#[derive(Debug)]
/// Base level translation table
pub struct TranslationTable {
    pointer: *mut TranslationTableMemory,
}

impl TranslationTable {
    /// Create a pointer to a base table from a virtual address
    pub const fn new(pointer: *mut TranslationTableMemory) -> Self {
        Self { pointer }
    }
    /// Registers the translation page table as ttbr0
    ///
    /// # Safety
    /// The caller must garantee that the new translation page table is valid
    pub unsafe fn set_as_ttbr0(&self) -> Result<()> {
        let virt_addr = VirtualAddress::from_ptr(self.pointer);
        let phys_addr = get_phys_addr(virt_addr)?;
        TTBR0.set(phys_addr.as_u32());
        arm::__nop();
        arm::__nop();
        arm::__nop();
        Ok(())
    }

    /// This functions is deprecated since it assumes that the ttbr0 is on indentity-mapped memory
    /// address
    /// Resolution: Use get_phys_addr() and convert it to a virtual memory address
    pub fn get_current_ttbr0() -> Self {
        let ttbr0 = TTBR0.get() & !0x3fff;
        TranslationTable {
            pointer: ttbr0 as *mut _,
        }
    }
    /// Get the ttbr0 translation table
    pub fn get_ttbr0(offset_mapping: OffsetMapping) -> Result<Self> {
        let phys_addr = Self::get_ttbr0_phys();
        let virt_addr = offset_mapping.convert_phys_addr(phys_addr)?;
        Ok(Self::new(virt_addr.as_mut_ptr()))
    }

    /// Returns the physical address of the translation page table
    pub fn get_ttbr0_phys() -> PhysicalAddress {
        PhysicalAddress::new(TTBR0.get() & !0x3fff)
    }

    /// Mutable reference to the translation table
    ///
    /// # Safety
    /// The caller must ensure that all entries are valid and don't interfere with memory layout of
    /// the program
    pub unsafe fn table_mut(
        &mut self,
    ) -> &mut [TranslationTableDescriptor; TRANSLATION_TABLE_SIZE] {
        &mut (*self.pointer).table
    }

    /// Immutable reference to the translation table
    ///
    pub fn table(&self) -> &[TranslationTableDescriptor; TRANSLATION_TABLE_SIZE] {
        unsafe { &(*self.pointer).table }
    }
}

impl fmt::LowerHex for TranslationTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.pointer as usize as u32;
        fmt::LowerHex::fmt(&val, f)
    }
}

//
// Page table stuff
//
//

#[derive(Copy, Clone, Debug, PartialEq)]
/// Different types of entries in a page table
pub enum PageTableType {
    Invalid,
    SmallPage,
    LargePage,
}

impl PageTableType {
    fn align(self) -> u32 {
        match self {
            PageTableType::Invalid => 0,
            PageTableType::SmallPage => 0xfff,
            PageTableType::LargePage => 0xffff,
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
/// A descriptor for a page table entry
pub struct PageTableDescriptor(u32);

impl fmt::LowerHex for PageTableDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val, f)
    }
}

impl ops::BitOr<u32> for PageTableDescriptor {
    type Output = Self;
    fn bitor(self, rhs: u32) -> Self {
        PageTableDescriptor(self.0 | rhs)
    }
}
impl ops::BitOrAssign<u32> for PageTableDescriptor {
    fn bitor_assign(&mut self, rhs: u32) {
        self.0 |= rhs;
    }
}

impl PageTableDescriptor {
    /// Construct a new invalid page descriptor
    pub const fn new_empty() -> PageTableDescriptor {
        PageTableDescriptor(0)
    }
    /// Construct a new page descriptor
    pub fn new(
        pagetype: PageTableType,
        addr: PhysicalAddress,
        attributes: MemoryAttributes,
    ) -> Result<PageTableDescriptor> {
        if pagetype == PageTableType::Invalid {
            return Ok(PageTableDescriptor(0));
        }
        addr.check_align(pagetype.align())?;
        let mut out = attributes.to_page_descriptor(pagetype);
        out |= addr.0;
        Ok(out)
    }
    /// Determine the type of the page descriptor
    pub fn get_type(self) -> PageTableType {
        // starts with
        // 0b00 Invalid
        // 0b01 LargePage
        // 0b1x SmallPage
        match self.0 & 0b11 {
            0b00 => PageTableType::Invalid,
            0b01 => PageTableType::LargePage,
            _ => PageTableType::SmallPage,
        }
    }
    /// Get the physical base address the page is pointing to.
    pub fn get_addr(self) -> Result<PhysicalAddress> {
        let page_type = self.get_type();
        if page_type == PageTableType::Invalid {
            return Err(PageError::InvalidMemory);
        }
        let strip_addr = self.0 & (!page_type.align());
        Ok(PhysicalAddress(strip_addr))
    }
}

pub const PAGE_TABLE_SIZE: usize = 256; // each page is 4Kb, section is 1MB; 1Mb / 4Kb = 256

#[repr(C)]
#[repr(align(1024))]
/// Second level page table memory
pub struct PageTableMemory {
    table: [PageTableDescriptor; PAGE_TABLE_SIZE],
}
impl PageTableMemory {
    pub const fn new() -> Self {
        Self {
            table: [PageTableDescriptor::new_empty(); PAGE_TABLE_SIZE],
        }
    }
}

#[derive(Debug)]
/// Second level page table
pub struct PageTable {
    pointer: *mut PageTableMemory,
    descriptor: TranslationTableDescriptor,
}

impl PageTable {
    /// Creates a page table and registers it in the translation table at a given index
    ///
    /// # Safety
    /// Any interaction with the translation table is unsafe as it might corrupt data rust is
    /// interacting with.
    pub unsafe fn new(
        pointer: *mut PageTableMemory,
        mem_attributes: MemoryAttributes,
        base_table: &mut TranslationTable,
        index: usize,
    ) -> Result<Self> {
        let virt_addr = VirtualAddress::from_mut_ptr(pointer);
        let phys_addr = get_phys_addr(virt_addr)?;
        let descriptor =
            TranslationTableDescriptor::new(TranslationTableType::Page, phys_addr, mem_attributes)?;
        base_table.table_mut()[index] = descriptor;
        let page_table = PageTable {
            pointer,
            descriptor,
        };
        Ok(page_table)
    }
    /// Mutable reference to the page table
    ///
    /// # Safety
    /// The caller must ensure that all entries are valid and don't interfere with memory layout of
    /// the program
    pub unsafe fn table_mut(&mut self) -> &mut [PageTableDescriptor; PAGE_TABLE_SIZE] {
        &mut (*self.pointer).table
    }
    /// Immutable reference to the page table
    pub fn table(&self) -> &[PageTableDescriptor; PAGE_TABLE_SIZE] {
        unsafe { &(*self.pointer).table }
    }
}
