//! Register access to the program state registers
//!
//! # Usage examples
//! Reading the current mode:
//! ```
//!     CPSR.read_as_enum(PSR::MODE);
//! ```
//! Read masking of IRQ:
//! ```
//!     CPSR.is_set(PSR::IRQ);
//! ```
//! Enable FIQs
//! ```
//!     CPSR.modify(PSR::FIQ::NotMasked);
//! ```

use core::fmt;
use register::{cpu::RegisterReadWrite, register_bitfields, FieldValue};

register_bitfields! {u32,
    pub PSR [
        MODE OFFSET(0) NUMBITS(5) [
            USR = 0b10000,
            FIQ = 0b10001,
            IRQ = 0b10010,
            SVC = 0b10011,
            MON = 0b10110,
            ABT = 0b10111,
            HYP = 0b11010,
            UND = 0b11011,
            SYS = 0b11111
        ],
        THUMB OFFSET(5) NUMBITS(1) [Thumb = 1, Arm = 0],
        //MASK OFFSET(6) NUMBITS(3) [ ],
        FIQ OFFSET(6) NUMBITS(1) [Masked = 1, NotMasked = 0],
        IRQ OFFSET(7) NUMBITS(1) [Masked = 1, NotMasked = 0],
        ABT OFFSET(8) NUMBITS(1) [Masked = 1, NotMasked = 0],
        ENDIAN OFFSET(9) NUMBITS(1) [Little = 0b0, Big = 0b1],
        JAZELLE OFFSET(24) NUMBITS(1) []
    ]
}

pub fn set_current_mode(mode: FieldValue<u32, PSR::Register>) {
    CPSR.modify(mode);
}

impl fmt::Display for PSR::MODE::Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = match &self {
            PSR::MODE::Value::USR => "User (USR)",
            PSR::MODE::Value::SYS => "System (SYS)",
            PSR::MODE::Value::IRQ => "Interrupt (IRQ)",
            PSR::MODE::Value::FIQ => "Interrupt (FIQ)",
            PSR::MODE::Value::SVC => "Supervisor (SVC)",
            PSR::MODE::Value::UND => "Undefined (UND)",
            PSR::MODE::Value::ABT => "Abort (ABT)",
            PSR::MODE::Value::HYP => "Hypervisor (HYP)",
            PSR::MODE::Value::MON => "Monitor (MON)",
        };
        f.write_str(string)
    }
}

pub fn get_current_mode() -> Option<PSR::MODE::Value> {
    CPSR.read_as_enum(PSR::MODE)
}

pub struct CurrentProgramState;

impl RegisterReadWrite<u32, PSR::Register> for CurrentProgramState {
    psr_read_raw!(u32, "cpsr");
    psr_write_raw!(u32, "cpsr");
}

pub struct SavedProgramState;

impl RegisterReadWrite<u32, PSR::Register> for SavedProgramState {
    psr_read_raw!(u32, "spsr");
    psr_write_raw!(u32, "spsr");
}

/// Current Program State register
pub static CPSR: CurrentProgramState = CurrentProgramState {};
/// Saved Program State register
pub static SPSR: SavedProgramState = SavedProgramState {};
