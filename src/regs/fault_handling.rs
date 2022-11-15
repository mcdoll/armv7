//! Register access to the PL1 fault handling functional group
//!
//! # Usage examples

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_bitfields;

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

impl Readable for DataFaultAddress {
    type T = u32;
    type R = ();

    sys_coproc_read_raw!(u32, "p15", "c6", "c0", "0", "0");
}

impl Writeable for DataFaultAddress {
    type T = u32;
    type R = ();

    sys_coproc_write_raw!(u32, "p15", "c6", "c0", "0", "0");
}

impl Readable for DataFaultStatus {
    type T = u32;
    type R = DFS::Register;

    sys_coproc_read_raw!(u32, "p15", "c5", "c0", "0", "0");
}

impl Writeable for DataFaultStatus {
    type T = u32;
    type R = DFS::Register;

    sys_coproc_write_raw!(u32, "p15", "c5", "c0", "0", "0");
}

/// Public interface for the DFAR
pub static DFAR: DataFaultAddress = DataFaultAddress {};
/// Public interface for the DFAR
pub static DFSR: DataFaultStatus = DataFaultStatus {};
