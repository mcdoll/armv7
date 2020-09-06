//! Register access to the PL1 fault handling functional group
//!
//! # Usage examples

pub use register::cpu::RegisterReadWrite;
use register::register_bitfields;

register_bitfields! {u32,
    pub DFS [
        FS OFFSET(0) NUMBITS(4) [],
        DOMAIN OFFSET(4) NUMBITS(4) [],
        LPAE OFFSET(9) NUMBITS(1) [],
        FS4 OFFSET(10) NUMBITS(1) [],
        WNR OFFSET(11) NUMBITS(1) [],
        EXT OFFSET(12) NUMBITS(1) [],
        CM OFFSET(13) NUMBITS(1) []
    ]
}
pub struct DataFaultAddress;
pub struct DataFaultStatus;
pub struct InstructionFaultAddress;
pub struct InstructionFaultStatus;

impl RegisterReadWrite<u32, ()> for DataFaultAddress {
    sys_coproc_read_raw!(u32, "p15", "c6", "c0", "0", "0");
    sys_coproc_write_raw!(u32, "p15", "c6", "c0", "0", "0");
}
impl RegisterReadWrite<u32, DFS::Register> for DataFaultStatus {
    sys_coproc_read_raw!(u32, "p15", "c5", "c0", "0", "0");
    sys_coproc_write_raw!(u32, "p15", "c5", "c0", "0", "0");
}

/// Public interface for the DFAR
pub static DFAR: DataFaultAddress = DataFaultAddress {};
/// Public interface for the DFAR
pub static DFSR: DataFaultStatus = DataFaultStatus {};
