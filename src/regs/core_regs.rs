//! Access to the core registers of armv7
// Author: Moritz Doll
// License: MIT

use register::cpu::RegisterReadWrite;

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

/// Stack pointer
pub static PC: ProgramCounter = ProgramCounter {};

/// Stack pointer
pub static SP: StackPointer = StackPointer {};


