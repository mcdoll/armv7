//! Page table related


use crate::regs::vmem_control::*;
//use crate::regs::ats;
use core::fmt;
use core::ops;
use core::arch::arm;
use crate::{VirtualAddress,PhysicalAddress};

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

//const ALIGN4KB: u32 = 0xffff_c000;

trait Alignable {
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
        self.0 = self.0 | !mask;
    }
}
impl Alignable for PhysicalAddress {
    fn is_aligned(&self, mask: u32) -> bool {
        (self.0 & mask) == 0
    }
    fn align(&mut self, mask: u32) {
        self.0 = self.0 | !mask;
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
    pub fn new(virt: VirtualAddress, phys: PhysicalAddress, size: u32) -> Self {
        OffsetMapping {
            virt_start: virt,
            phys_start: phys,
            size: size,
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
    fn align(&self) -> u32 {
        match self {
            TableType::Invalid => 0,
            TableType::Page => 0x3ff,
            TableType::Section => 0xf_ffff,
            TableType::Supersection => 0xf_ffff,
        }
    }
    fn bitset(&self) -> u32 {
        match self {
            TableType::Invalid => 0,
            TableType::Page => 0b01,
            TableType::Section => 0b10,
            TableType::Supersection => 0x4_0002,
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
        self.0 = self.0 | !mask;
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
    pub fn get_type(&self) -> TableType {
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
    pub fn new(tabletype: TableType, addr: PhysicalAddress) -> Result<TableDescriptor> {
        if tabletype == TableType::Invalid {
            return Ok(TableDescriptor(0))
        }
        addr.check_align(tabletype.align())?;
        let bitset = tabletype.bitset();
        Ok(TableDescriptor(addr.0 | bitset))
    }
    pub fn new_pagetable(addr: PhysicalAddress, domain: u8, ns: bool, pxn: bool) -> Result<TableDescriptor> {
        let mut output = Self::new(TableType::Page, addr)?; 
        let domain_u32 = domain as u32;
        output |= domain_u32 << 5;
        if pxn {
            output |= 1 << 2;
        }
        if ns {
            output |= 1 << 3;
        }
        Ok(output)
    }
}

#[derive(Copy,Clone,Debug,PartialEq)]
pub enum PageType {
    Invalid,
    SmallPage,
    LargePage,
}

impl PageType {
   fn align(&self) -> u32 {
        match self {
            PageType::Invalid => 0,
            PageType::SmallPage => 0xfff,
            PageType::LargePage => 0xffff,
        }
    }
    fn bitset(&self) -> u32 {
        match self {
            PageType::Invalid => 0,
            PageType::SmallPage => 0b10,
            PageType::LargePage => 0b01,
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
        self.0 = self.0 | !mask;
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
    /// Determine the type of the page descriptor
    pub fn get_type(&self) -> PageType {
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
    pub fn get_addr(&self) -> Result<PhysicalAddress> {
        let page_type = self.get_type();
        if page_type == PageType::Invalid { return Err(PageError::InvalidMemory) }
        let strip_addr = self.0 & ( !page_type.align() );
        Ok( PhysicalAddress ( strip_addr ) )
    }
    /// Construct a new page descriptor
    pub fn new(pagetype: PageType, addr: PhysicalAddress) -> Result<PageDescriptor> {
        if pagetype == PageType::Invalid {
            return Ok(PageDescriptor(0))
        }
        addr.check_align(pagetype.align())?;
        let bitset = pagetype.bitset();
        Ok(PageDescriptor(addr.as_u32() | bitset))
    }
    pub fn new_smallpage(addr: PhysicalAddress, ap: u8, tex: u8, xn: bool, c: bool, b: bool, ng: bool, s: bool) -> Result<PageDescriptor> {
        let mut output = Self::new(PageType::SmallPage, addr)?;
        let tex_u32 = tex as u32;
        //let ap_u32 = ap as u32;
        output |= tex_u32 << 6;
        let ap_2 = ((ap & 0b100) >> 2) as u32;
        let ap_01 = (ap & 0b011) as u32;
        output |= (ap_2 << 9) | (ap_01 << 4);
        if xn { output |= 1; }
        if b { output |= 1 << 2; }
        if c { output |= 1 << 3; }
        if ng { output |= 1 << 11; }
        if s { output |= 1 << 10; }
        Ok(output)
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
    domain: u8,
    priv_exec_never: bool,
    non_secure: bool,
}

impl PageTable {
    /// Create a pointer to a second level page table from a virtual address
    /// The caller must garantee that the virtual address maps to a second level page table
    pub unsafe fn new(virt_addr: VirtualAddress) -> Result<Self> {
        virt_addr.check_align(0x3ff)?;
        Ok(PageTable {
            pointer: virt_addr.as_u32() as *mut _,
            domain: 0,
            priv_exec_never: false,
            non_secure: false,
        })
    }
    fn ptr(&self) -> *mut PageTableMemory {
        self.pointer
    }
    pub unsafe fn create(virt_addr: VirtualAddress, index: usize, base_table: &mut TranslationTable) -> Result<Self> {
        let table = Self::new(virt_addr)?;
        let descriptor = table.get_descriptor()?;
        base_table[index] = descriptor;
        Ok(table)
    }
    pub fn get_descriptor(&self) -> Result<TableDescriptor> {
        //check_align(addr, pagetype.align())?;
        let virt_addr = VirtualAddress::from_mut_ptr(self.pointer);
        let phys_addr = get_phys_addr(virt_addr)?;
        TableDescriptor::new(TableType::Page, phys_addr)
    }
    /*fn set_page(&mut self, entry: usize, page: PageDescriptor) {
        self[entry] = page;
    }*/
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

