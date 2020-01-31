// Author: Moritz Doll
// License: MIT

//! Register access to the security extension registers
//!
//! Functional group according to the ARM ARM

use register::cpu::*;

pub struct InterruptStatus;
pub struct MonitorVectorBaseAddress;
pub struct NonSecureAccessControl;
pub struct SecureConfiguration;
pub struct SecureDebugEnable;
pub struct VectorBaseAddress;

impl RegisterReadOnly<u32, ()> for InterruptStatus {
    sys_coproc_read_raw!(u32, "p15", "c12", "c1", "0", "0");
}

impl RegisterReadWrite<u32, ()> for MonitorVectorBaseAddress {
    sys_coproc_read_raw!(u32, "p15", "c12", "c0", "0", "1");
    sys_coproc_write_raw!(u32, "p15", "c12", "c0", "0", "1");
}

impl RegisterReadWrite<u32, ()> for NonSecureAccessControl {
    sys_coproc_read_raw!(u32, "p15", "c1", "c1", "0", "2");
    sys_coproc_write_raw!(u32, "p15", "c1", "c1", "0", "2");
}

impl RegisterReadWrite<u32, ()> for SecureConfiguration {
    sys_coproc_read_raw!(u32, "p15", "c1", "c1", "0", "0");
    sys_coproc_write_raw!(u32, "p15", "c1", "c1", "0", "0");
}

impl RegisterReadWrite<u32, ()> for SecureDebugEnable {
    sys_coproc_read_raw!(u32, "p15", "c1", "c1", "0", "1");
    sys_coproc_write_raw!(u32, "p15", "c1", "c1", "0", "1");
}

impl RegisterReadWrite<u32, ()> for VectorBaseAddress {
    sys_coproc_read_raw!(u32, "p15", "c12", "c0", "0", "0");
    sys_coproc_write_raw!(u32, "p15", "c12", "c0", "0", "0");
}

pub static ISR: InterruptStatus = InterruptStatus {};
pub static MVBAR: MonitorVectorBaseAddress = MonitorVectorBaseAddress {};
pub static NSACR: NonSecureAccessControl = NonSecureAccessControl {};
pub static SCR: SecureConfiguration = SecureConfiguration {};
pub static SDER: SecureDebugEnable = SecureDebugEnable {};
pub static VBAR: VectorBaseAddress = VectorBaseAddress {};
