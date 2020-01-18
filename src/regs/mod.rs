//! Processor core registers
//
// Author: Moritz Doll
// License: MIT

// The naming scheme and the submodules are according to the ARM Architecture Reference manual.
// The name of the struct is the description in ARM ARM and the static instance is name
// The submodules are given by the functional group
#[macro_use]
mod macros;

pub mod core_regs; // this is called core_regs to avoid a name clash with the core crate
pub mod program_state;
pub mod vmem_control;
pub mod address_translation;
pub mod security;

pub use register::cpu::*;

// General registers
pub use self::core_regs::SP;
pub use self::core_regs::PC;
pub use self::program_state::CPSR;
pub use self::program_state::SPSR;

// VMSA registers in CP14 and CP15
pub use self::vmem_control::SCTLR;
pub use self::vmem_control::TTBR0;
pub use self::vmem_control::TTBR1;
pub use self::security::ISR;
pub use self::security::MVBAR;
pub use self::security::NSACR;
pub use self::security::SCR;
pub use self::security::SDER;
pub use self::security::VBAR;
