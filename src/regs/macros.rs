// Module taken from Cortex-A crate by Andre Richter

macro_rules! __read_raw {
    ($width:ty, $asm_instr:tt, $asm_reg_name:tt) => {
        /// Reads the raw bits of the CPU register.
        #[inline]
        fn get(&self) -> $width {
            match () {
                #[cfg(target_arch = "arm")]
                () => {
                    let reg;
                    unsafe {
                        core::arch::asm!(
                            concat!($asm_instr, " {value}, ", $asm_reg_name),
                            value = out(reg) reg
                        );
                    }
                    reg
                }

                #[cfg(not(target_arch = "arm"))]
                () => unimplemented!(),
            }
        }
    };
}

macro_rules! __write_raw {
    ($width:ty, $asm_instr:tt, $asm_reg_name:tt) => {
        /// Writes raw bits to the CPU register.
        #[cfg_attr(not(target_arch = "arm"), allow(unused_variables))]
        #[inline]
        fn set(&self, value: $width) {
            match () {
                #[cfg(target_arch = "arm")]
                () => {
                    unsafe {
                        core::arch::asm!(
                            concat!($asm_instr, " ", $asm_reg_name, ", {value}"),
                            value = in(reg) value
                        )
                    }
                }

                #[cfg(not(target_arch = "arm"))]
                () => unimplemented!(),
            }
        }
    };
}

/// Raw read from system coprocessor registers.
/// Arguments are the coprocessor, CRn, CRm, Opcode1, Opcode2
macro_rules! sys_coproc_read_raw {
    ($width:ty, $asm_cp:tt, $asm_crn:tt, $asm_crm:tt, $asm_opc1:tt, $asm_opc2:tt) => {
        /// Reads the raw bits of the CPU register.
        #[inline]
        fn get(&self) -> $width {
            match () {
                #[cfg(target_arch = "arm")]
                () => {
                    let reg;
                    unsafe {
                        // MRC<c> <coproc>, <opc1>, <Rt>, <CRn>, <CRm>{, <opc2>}
                        core::arch::asm!(
                            concat!("mrc ", $asm_cp, ", ", $asm_opc1, ", {reg}, ", $asm_crn, ", ", $asm_crm, ", ", $asm_opc2),
                            reg = out(reg) reg
                        );
                    }
                    reg
                }

                #[cfg(not(target_arch = "arm"))]
                () => unimplemented!(),
            }
        }
    };
}

/// Raw write to system coprocessor registers.
/// Arguments are the coprocessor, CRn, CRm, Opcode1, Opcode2
macro_rules! sys_coproc_write_raw {
    ($width:ty, $asm_cp:tt, $asm_crn:tt, $asm_crm:tt, $asm_opc1:tt, $asm_opc2:tt) => {
        /// Writes raw bits to the CPU register.
        #[cfg_attr(not(target_arch = "arm"), allow(unused_variables))]
        #[inline]
        fn set(&self, value: $width) {
            match () {
                #[cfg(target_arch = "arm")]
                () => {
                    unsafe {
                        // MCR<c> <coproc>, <opc1>, <Rt>, <CRn>, <CRm>{, <opc2>}
                        core::arch::asm!(
                            concat!("mcr ", $asm_cp, ", ", $asm_opc1, ", {value}, ", $asm_crn, ", ", $asm_crm, ", ", $asm_opc2),
                            value = in(reg) value
                        );
                    }
                }

                #[cfg(not(target_arch = "arm"))]
                () => unimplemented!(),
            }
        }
    };
}

/// Raw read from system coprocessor registers.
macro_rules! psr_read_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __read_raw!($width, "mrs", $asm_reg_name);
    };
}

/// Raw write to system coprocessor registers.
macro_rules! psr_write_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __write_raw!($width, "msr", $asm_reg_name);
    };
}

/// Raw read from (ordinary) registers
macro_rules! read_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __read_raw!($width, "mov", $asm_reg_name);
    };
}
/// Raw write to (ordinary) registers
macro_rules! write_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __write_raw!($width, "mov", $asm_reg_name);
    };
}
