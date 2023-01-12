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

use core::arch::asm;
use core::fmt;
use core::ops;

use tock_registers::register_bitfields;

pub use tock_registers::interfaces::{Readable, Writeable};
pub use tock_registers::registers::InMemoryRegister;

register_bitfields! {
    u32,
    pub PAGE_TABLE_FLAGS [
        VALID     OFFSET(0)  NUMBITS(1)  [Enable = 0b1],
        NS        OFFSET(3)  NUMBITS(1)  [Enable = 0b1],
        DOMAIN    OFFSET(5)  NUMBITS(3)  [],
        ADDR      OFFSET(10) NUMBITS(12) []
    ]
}

register_bitfields! {
    u32,
    pub SMALL_PAGE_FLAGS [
        XN    OFFSET(0)  NUMBITS(1)  [Enable = 0b1],
        VALID OFFSET(1)  NUMBITS(1)  [Enable = 0b1],
        B     OFFSET(2)  NUMBITS(1)  [Enable = 0b1],
        C     OFFSET(3)  NUMBITS(1)  [Enable = 0b1],
        AP    OFFSET(4)  NUMBITS(2)  [
            NoAccess = 0b00,
            PrivAccess = 0b01,
            UnprivReadOnly = 0b10,
            FullAccess = 0b11
        ],
        TEX   OFFSET(6)  NUMBITS(3)  [],
        AP2   OFFSET(9)  NUMBITS(1)  [Enable = 0b1],
        S     OFFSET(10) NUMBITS(1)  [Enable = 0b1],
        NG    OFFSET(11) NUMBITS(1)  [Enable = 0b1],
        ADDR  OFFSET(12) NUMBITS(20) []
    ]
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

pub type Result<T> = core::result::Result<T, PageError>;

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
            asm!("mcr p15, 0, {}, c7, c8, 0", in(reg) virt_addr.as_u32())
        }
        (true, true) => {
            asm!("mcr p15, 0, {}, c7, c8, 1", in(reg) virt_addr.as_u32())
        }
        (false, false) => {
            asm!("mcr p15, 0, {}, c7, c8, 2", in(reg) virt_addr.as_u32())
        }
        (false, true) => {
            asm!("mcr p15, 0, {}, c7, c8, 3", in(reg) virt_addr.as_u32())
        }
    }
    asm!("mrc p15, 0, {}, c7, c4, 0", out(reg) output);
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

impl fmt::Binary for TranslationTableDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::Binary::fmt(&val, f)
    }
}

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
        flags: u32,
    ) -> Result<Self> {
        if tabletype == TranslationTableType::Invalid {
            return Ok(TranslationTableDescriptor(0));
        }
        addr.check_align(tabletype.align())?;

        let out = addr.0 | flags;

        Ok(Self(out))
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

    pub fn get_addr(&self) -> Result<PhysicalAddress> {
        let entry_type = self.get_type();
        if entry_type == TranslationTableType::Invalid {
            return Err(PageError::InvalidMemory);
        }

        let strip_addr = self.0 & (!entry_type.align());
        Ok(PhysicalAddress(strip_addr))
    }

    pub fn as_u32(&self) -> u32 {
        self.0
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
        asm!("nop");
        asm!("nop");
        asm!("nop");
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

impl fmt::Binary for PageTableDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::Binary::fmt(&val, f)
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
        flags: u32,
    ) -> Result<PageTableDescriptor> {
        if pagetype == PageTableType::Invalid {
            return Ok(PageTableDescriptor(0));
        }
        addr.check_align(pagetype.align())?;
        let out = addr.0 | flags;

        Ok(Self(out))
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

    pub fn as_u32(&self) -> u32 {
        self.0
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

    #[allow(dead_code)]
    descriptor: Option<TranslationTableDescriptor>,
}

impl PageTable {
    pub unsafe fn new_from_ptr(pointer: *mut PageTableMemory) -> Self {
        Self {
            pointer,
            descriptor: None,
        }
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
