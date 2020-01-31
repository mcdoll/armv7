//! Crate for low-level armv7 routines

#![feature(asm)]
#![feature(stdsimd)]
#![no_std]
#![feature(const_fn)]

use core::fmt;
use core::ops::{Add, AddAssign, BitOr, Sub, SubAssign};

//pub mod asm;
pub mod regs;
pub mod structures;

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct VirtualAddress(u32);

impl VirtualAddress {
    /// Create a new virtual address
    pub const fn new(addr: u32) -> VirtualAddress {
        VirtualAddress(addr)
    }
    /// Create a virtual address from the indices of a translation table and a page table and the
    /// offset
    ///
    pub fn from_indices(
        translation_index: usize,
        page_index: usize,
        offset: u32,
    ) -> Option<VirtualAddress> {
        use structures::paging::*;
        // The address is build as follows
        // 0xXXXY_YZZZ,
        // where
        // Xs are the translation table index,
        // Ys are the page table index
        // Zs are the offset
        if (translation_index >= TRANSLATION_TABLE_SIZE)
            || (page_index >= PAGE_TABLE_SIZE)
            || offset >= 0xfff
        {
            return None;
        };
        let mut address = (translation_index as u32) << 20;
        address |= (page_index as u32) << 12;
        address |= offset;
        Some(VirtualAddress(address))
    }
    /// Calculate for a virtual address the index in the base table
    pub const fn base_table_index(self) -> usize {
        // Divide by 1Mb
        let base_addr = self.0 >> 20;
        base_addr as usize
    }
    /// Calculate the index in a page table
    pub const fn page_table_index(self) -> usize {
        let page_addr = (self.0 & 0xfffff) >> 12;
        page_addr as usize
    }
    /// Calculate the offset in a page table
    pub const fn page_table_offset(self) -> u32 {
        self.0 | 0xfff
    }
    /// Converts the address to an unsigned integer
    pub fn as_u32(self) -> u32 {
        self.0
    }
    /// Create a virtual address from a pointer
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as usize as u32)
    }
    /// Create a virtual address from a mutable pointer
    pub fn from_mut_ptr<T>(ptr: *mut T) -> Self {
        Self::new(ptr as usize as u32)
    }
    /// Creates a virtual address from an extern "C" fn
    pub fn from_ext_fn(fn_ptr: extern "C" fn() -> !) -> Self {
        Self::new(fn_ptr as usize as u32)
    }
    /// Converts the address to a raw pointer
    #[cfg(target_pointer_width = "32")]
    pub fn as_ptr<T>(self) -> *const T {
        self.as_u32() as usize as *const T
    }
    /// Converts the address to a mutable raw pointer
    #[cfg(target_pointer_width = "32")]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }
}

impl Add<u32> for VirtualAddress {
    type Output = Self;
    fn add(self, rhs: u32) -> Self::Output {
        VirtualAddress::new(self.0 + rhs)
    }
}

impl AddAssign<u32> for VirtualAddress {
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}

impl BitOr<u32> for VirtualAddress {
    type Output = Self;
    fn bitor(self, rhs: u32) -> Self::Output {
        VirtualAddress::new(self.0 | rhs)
    }
}

#[cfg(target_pointer_width = "32")]
impl Add<usize> for VirtualAddress {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        self + rhs as u32
    }
}

#[cfg(target_pointer_width = "32")]
impl AddAssign<usize> for VirtualAddress {
    fn add_assign(&mut self, rhs: usize) {
        self.add_assign(rhs as u32)
    }
}

impl Sub<u32> for VirtualAddress {
    type Output = Self;
    fn sub(self, rhs: u32) -> Self::Output {
        VirtualAddress::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<u32> for VirtualAddress {
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}

#[cfg(target_pointer_width = "32")]
impl Sub<usize> for VirtualAddress {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self::Output {
        self - rhs as u32
    }
}

#[cfg(target_pointer_width = "32")]
impl SubAssign<usize> for VirtualAddress {
    fn sub_assign(&mut self, rhs: usize) {
        self.sub_assign(rhs as u32)
    }
}

impl Sub<VirtualAddress> for VirtualAddress {
    type Output = u32;
    fn sub(self, rhs: VirtualAddress) -> Self::Output {
        self.as_u32().checked_sub(rhs.as_u32()).unwrap()
    }
}

impl fmt::LowerHex for VirtualAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val, f)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct PhysicalAddress(u32);

impl PhysicalAddress {
    /// Create a new physical address from an unsigned integer
    pub const fn new(addr: u32) -> PhysicalAddress {
        PhysicalAddress(addr)
    }
    /// Converts the address to an unsigned integer
    pub const fn as_u32(self) -> u32 {
        self.0
    }
    /// Create a physical address from a pointer
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as usize as u32)
    }
    /// Converts the address to a raw pointer
    #[cfg(target_pointer_width = "32")]
    pub fn as_ptr<T>(self) -> *const T {
        self.as_u32() as usize as *const T
    }
    /// Creates a physical address from an extern "C" fn
    pub fn from_ext_fn(fn_ptr: extern "C" fn() -> !) -> Self {
        Self::new(fn_ptr as usize as u32)
    }
    /// Converts the address to a mutable raw pointer
    #[cfg(target_pointer_width = "32")]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }
}

impl Add<u32> for PhysicalAddress {
    type Output = Self;
    fn add(self, rhs: u32) -> Self::Output {
        PhysicalAddress::new(self.0 + rhs)
    }
}

impl AddAssign<u32> for PhysicalAddress {
    fn add_assign(&mut self, rhs: u32) {
        *self = *self + rhs;
    }
}

#[cfg(target_pointer_width = "32")]
impl Add<usize> for PhysicalAddress {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        self + rhs as u32
    }
}

#[cfg(target_pointer_width = "32")]
impl AddAssign<usize> for PhysicalAddress {
    fn add_assign(&mut self, rhs: usize) {
        self.add_assign(rhs as u32)
    }
}

impl Sub<u32> for PhysicalAddress {
    type Output = Self;
    fn sub(self, rhs: u32) -> Self::Output {
        PhysicalAddress::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<u32> for PhysicalAddress {
    fn sub_assign(&mut self, rhs: u32) {
        *self = *self - rhs;
    }
}

#[cfg(target_pointer_width = "32")]
impl Sub<usize> for PhysicalAddress {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self::Output {
        self - rhs as u32
    }
}

#[cfg(target_pointer_width = "32")]
impl SubAssign<usize> for PhysicalAddress {
    fn sub_assign(&mut self, rhs: usize) {
        self.sub_assign(rhs as u32)
    }
}

impl Sub<PhysicalAddress> for PhysicalAddress {
    type Output = u32;
    fn sub(self, rhs: PhysicalAddress) -> Self::Output {
        self.as_u32().checked_sub(rhs.as_u32()).unwrap()
    }
}

impl fmt::LowerHex for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = self.0;
        fmt::LowerHex::fmt(&val, f)
    }
}
