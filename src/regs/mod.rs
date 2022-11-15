//! Processor registers

// The naming scheme and the submodules are according to the ARM Architecture Reference manual.
// The name of the struct is the description in ARM ARM and the static instance is name
// The submodules are given by the functional group
#[macro_use]
mod macros;

pub mod address_translation;
pub mod core_regs; // this is called core_regs to avoid a name clash with the core crate
pub mod fault_handling;
pub mod program_state;
pub mod security;
pub mod vmem_control;

