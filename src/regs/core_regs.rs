//! Register access to the core registers

use register::cpu::RegisterReadWrite;
use register::InMemoryRegister;
use register::*;

use crate::regs::program_state::PSR;
use crate::VirtualAddress;

use crate::fmt;

pub struct ProgramCounter;
pub struct StackPointer;

impl RegisterReadWrite<u32, ()> for ProgramCounter {
    read_raw!(u32, "pc");
    write_raw!(u32, "pc");
}

impl RegisterReadWrite<u32, ()> for StackPointer {
    read_raw!(u32, "sp");
    write_raw!(u32, "sp");
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct MemoryRegister<R: RegisterLongName>(InMemoryRegister<u32, R>);

impl<R: RegisterLongName> MemoryRegister<R> {
    pub const fn new(value: u32) -> Self {
        MemoryRegister(InMemoryRegister::new(value))
    }
}
impl<R: RegisterLongName> fmt::Debug for MemoryRegister<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.get().fmt(f)
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CoreRegisters {
    pub r0: MemoryRegister<()>,  // 0x00
    pub r1: MemoryRegister<()>,  // 0x04
    pub r2: MemoryRegister<()>,  // 0x08
    pub r3: MemoryRegister<()>,  // 0x0C
    pub r4: MemoryRegister<()>,  // 0x10
    pub r5: MemoryRegister<()>,  // 0x14
    pub r6: MemoryRegister<()>,  // 0x18
    pub r7: MemoryRegister<()>,  // 0x1C
    pub r8: MemoryRegister<()>,  // 0x20
    pub r9: MemoryRegister<()>,  // 0x24
    pub r10: MemoryRegister<()>, // 0x28
    pub fp: MemoryRegister<()>,  // 0x2C
    pub ip: MemoryRegister<()>,  // 0x30
    pub sp: MemoryRegister<()>,  // 0x34
    pub lr: MemoryRegister<()>,  // 0x38
    pub pc: MemoryRegister<()>,  // 0x3C
    pub psr: MemoryRegister<PSR::Register>, // 0x40
}

impl CoreRegisters {
    pub const fn new(psr: u32, sp: u32, pc: u32, lr: u32) -> Self {
        Self {
            r0: MemoryRegister::new(0),
            r1: MemoryRegister::new(0),
            r2: MemoryRegister::new(0),
            r3: MemoryRegister::new(0),
            r4: MemoryRegister::new(0),
            r5: MemoryRegister::new(0),
            r6: MemoryRegister::new(0),
            r7: MemoryRegister::new(0),
            r8: MemoryRegister::new(0),
            r9: MemoryRegister::new(0),
            r10: MemoryRegister::new(0),
            fp: MemoryRegister::new(0),
            ip: MemoryRegister::new(0),
            sp: MemoryRegister::new(sp),
            lr: MemoryRegister::new(lr),
            pc: MemoryRegister::new(pc),
            psr: MemoryRegister::new(psr) }
    }
    // Todo PC is a virtual address, add similar methods for LR and SP
    pub fn set_pc(&mut self, value: VirtualAddress) {
        self.pc.0.set(value.as_u32());
    }
    pub fn get_pc(&mut self) -> VirtualAddress {
        VirtualAddress::new(self.pc.0.get())
    }
    pub fn set_sp(&mut self, value: VirtualAddress) {
        self.sp.0.set(value.as_u32());
    }
    pub unsafe fn set_psr(&mut self, value: u32) {
        self.psr.0.set(value);
    }
    pub fn mode(&self) -> Option<PSR::MODE::Value> {
        //let psr_reg = InMemoryRegister::new(self.psr);
        self.psr.0.read_as_enum(PSR::MODE)
    }
    //
}

/// Program counter
pub static PC: ProgramCounter = ProgramCounter {};

/// Stack pointer
pub static SP: StackPointer = StackPointer {};
