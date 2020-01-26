//! Register access to the address translation functional group
//!
//! Deprecated. Use structures::paging::get_phys_frame
//! instead
// Author: Moritz Doll
// License: MIT

use register::cpu::{RegisterReadWrite, RegisterWriteOnly};

pub struct Stage1CurrentStatePL1Read;
pub struct Stage1CurrentStatePL1Write;
pub struct Stage1CurrentStateUnpriviledgedRead;
pub struct Stage1CurrentStateUnpriviledgedWrite;
pub struct PhysicalAddress;

impl RegisterWriteOnly<u32, ()> for Stage1CurrentStatePL1Read {
    sys_coproc_write_raw!(u32, "p15", "c7", "c8", "0", "0");
}

impl RegisterWriteOnly<u32, ()> for Stage1CurrentStatePL1Write {
    sys_coproc_write_raw!(u32, "p15", "c7", "c8", "0", "1");
}

impl RegisterWriteOnly<u32, ()> for Stage1CurrentStateUnpriviledgedRead {
    sys_coproc_write_raw!(u32, "p15", "c7", "c8", "0", "2");
}

impl RegisterWriteOnly<u32, ()> for Stage1CurrentStateUnpriviledgedWrite {
    sys_coproc_write_raw!(u32, "p15", "c7", "c8", "0", "3");
}

impl RegisterReadWrite<u32, ()> for PhysicalAddress {
    sys_coproc_read_raw!(u32, "p15", "c7", "c4", "0", "0");
    sys_coproc_write_raw!(u32, "p15", "c7", "c4", "0", "0");
}

/// Public interface for the ATS1CPR
pub static ATS1CPR: Stage1CurrentStatePL1Read = Stage1CurrentStatePL1Read {};
/// Public interface for the ATS1CPW
pub static ATS1CPW: Stage1CurrentStatePL1Write = Stage1CurrentStatePL1Write {};
/// Public interface for the ATS1CUR
pub static ATS1CUR: Stage1CurrentStateUnpriviledgedRead = Stage1CurrentStateUnpriviledgedRead {};
/// Public interface for the ATS1CUW
pub static ATS1CUW: Stage1CurrentStateUnpriviledgedWrite = Stage1CurrentStateUnpriviledgedWrite {};

/// Public interface for the PAR
pub static PAR: PhysicalAddress = PhysicalAddress {};
