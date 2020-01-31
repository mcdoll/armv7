//! Register access to the virtual memory control functional group
//!
//! # Usage examples
//! Read the current translation table
//! ```
//!     TTBR0.get()
//! ```
//! Set the translation table
//! ```
//!     TTBR0.set(0x8000_5000);
//! ```
//! Enable the MMU
//! ```
//!     SCTLR.modify(SCTLR::MMU::Enable);
//! ```
// Author: Moritz Doll
// License: MIT

pub use register::cpu::RegisterReadWrite;
use register::register_bitfields;

register_bitfields! {u32,
    pub SCTLR [
        MMU OFFSET(0) NUMBITS(1) [Enable = 1, Disable = 0],
        CACHE OFFSET(2) NUMBITS(1) [Enable = 1, Disable = 0],
        INSTR OFFSET(12) NUMBITS(1) [Enable = 1, Disable = 0],
        VECTOR OFFSET(13) NUMBITS(1) [High = 1, Low = 0],
        ALIGN OFFSET(22) NUMBITS(1) [],
        VECENABLE OFFSET(24) NUMBITS(1) [UseVectorTable = 0, ImplementationDefined = 1],
        EXCENDIAN OFFSET(25) NUMBITS(1) [LittleEndian = 0, BigEndian = 1],
        NMFIQ OFFSET(27) NUMBITS(1) [AllowMaskedFIQ = 0, ForbidMaskedFIQ = 1],
        TEXREMAP OFFSET(28) NUMBITS(1) [Enable = 1, Disable = 0],
        ACCFLAG OFFSET(29) NUMBITS(1) [Enable = 1, Disable = 0],
        THUMBEXC OFFSET(30) NUMBITS(1) [Arm = 0, Thumb = 1]
    ]
}

pub struct SystemControl;
pub struct TranslationTableBase0;
pub struct TranslationTableBase1;

impl RegisterReadWrite<u32, SCTLR::Register> for SystemControl {
    sys_coproc_read_raw!(u32, "p15", "c1", "c0", "0", "0");
    sys_coproc_write_raw!(u32, "p15", "c1", "c0", "0", "0");
}
impl RegisterReadWrite<u32, ()> for TranslationTableBase0 {
    sys_coproc_read_raw!(u32, "p15", "c2", "c0", "0", "0");
    sys_coproc_write_raw!(u32, "p15", "c2", "c0", "0", "0");
}

impl RegisterReadWrite<u32, ()> for TranslationTableBase1 {
    sys_coproc_read_raw!(u32, "p15", "c2", "c0", "0", "1");
    sys_coproc_write_raw!(u32, "p15", "c2", "c0", "0", "1");
}

/// Public interface for the SCTLR
pub static SCTLR: SystemControl = SystemControl {};

/// Public interface for the TTBR0
pub static TTBR0: TranslationTableBase0 = TranslationTableBase0 {};

/// Public interface for the TTBR1
pub static TTBR1: TranslationTableBase1 = TranslationTableBase1 {};
