//! Register access to the security extension registers
//!
//! Functional group according to the ARM ARM

use tock_registers::interfaces::{Writeable, Readable};

pub struct InterruptStatus;
pub struct MonitorVectorBaseAddress;
pub struct NonSecureAccessControl;
pub struct SecureConfiguration;
pub struct SecureDebugEnable;
pub struct VectorBaseAddress;

impl Readable for InterruptStatus {
    type T = u32;
    type R = ();

    sys_coproc_read_raw!(u32, "p15", "c12", "c1", "0", "0");
}

impl Readable for MonitorVectorBaseAddress {
    type T = u32;
    type R = ();

    sys_coproc_read_raw!(u32, "p15", "c12", "c0", "0", "1");
}

impl Writeable for MonitorVectorBaseAddress {
    type T = u32;
    type R = ();

    sys_coproc_write_raw!(u32, "p15", "c12", "c0", "0", "1");
}

impl Readable for NonSecureAccessControl {
    type T = u32;
    type R = ();

    sys_coproc_read_raw!(u32, "p15", "c1", "c1", "0", "2");
}

impl Writeable for NonSecureAccessControl {
    type T = u32;
    type R = ();

    sys_coproc_write_raw!(u32, "p15", "c1", "c1", "0", "2");
}

impl Readable for SecureConfiguration {
    type T = u32;
    type R = ();

    sys_coproc_read_raw!(u32, "p15", "c1", "c1", "0", "0");
}

impl Writeable for SecureConfiguration {
    type T = u32;
    type R = ();

    sys_coproc_write_raw!(u32, "p15", "c1", "c1", "0", "0");
}

impl Readable for SecureDebugEnable {
    type T = u32;
    type R = ();

    sys_coproc_read_raw!(u32, "p15", "c1", "c1", "0", "1");
}

impl Writeable for SecureDebugEnable {
    type T = u32;
    type R = ();

    sys_coproc_write_raw!(u32, "p15", "c1", "c1", "0", "1");
}

impl Readable for VectorBaseAddress {
    type T = u32;
    type R = ();

    sys_coproc_read_raw!(u32, "p15", "c12", "c0", "0", "0");
}

impl Writeable for VectorBaseAddress {
    type T = u32;
    type R = ();

    sys_coproc_write_raw!(u32, "p15", "c12", "c0", "0", "0");
}

pub static ISR: InterruptStatus = InterruptStatus {};
pub static MVBAR: MonitorVectorBaseAddress = MonitorVectorBaseAddress {};
pub static NSACR: NonSecureAccessControl = NonSecureAccessControl {};
pub static SCR: SecureConfiguration = SecureConfiguration {};
pub static SDER: SecureDebugEnable = SecureDebugEnable {};
pub static VBAR: VectorBaseAddress = VectorBaseAddress {};
