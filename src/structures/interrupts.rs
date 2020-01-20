//! Module for creating interrupt handlers

//use core::default::Default;
//use core::ptr;
//use core::mem;
use core::ops;
use register::mmio::*;
use crate::VirtualAddress;
use crate::regs::security::*;
use crate::regs::vmem_control::*;


#[repr(C)]
struct RegisterBlock {
    reset_handler: ReadWrite<u32,()>,
    undef_handler: ReadWrite<u32,()>,
    swi_handler: ReadWrite<u32,()>,
    prefetch_handler: ReadWrite<u32,()>,
    data_handler: ReadWrite<u32,()>,
    hyp_handler: ReadWrite<u32,()>,
    irq_handler: ReadWrite<u32,()>,
    fiq_handler: ReadWrite<u32,()>,
    reset_addr: ReadWrite<u32,()>,
    undef_addr: ReadWrite<u32,()>,
    swi_addr: ReadWrite<u32,()>,
    prefetch_addr: ReadWrite<u32,()>,
    data_addr: ReadWrite<u32,()>,
    hyp_addr: ReadWrite<u32,()>,
    irq_addr: ReadWrite<u32,()>,
    fiq_addr: ReadWrite<u32,()>,
}

#[repr(transparent)]
struct VectorTableMemory {
    memory_addr: u32,
}

impl ops::Deref for VectorTableMemory {
    type Target = RegisterBlock;
    fn deref(&self) -> &Self::Target {
        unsafe {&*self.ptr() }
    }
}

impl VectorTableMemory {
    fn new() -> Self {
        let table_addr = if SCTLR.is_set(SCTLR::VECTOR) {
            0xffff_0000
        } else {
            // We might have to check whether this register is there
            VBAR.get()
        };
        VectorTableMemory { memory_addr: table_addr }
    }
    fn ptr(&self) -> *mut RegisterBlock {
        self.memory_addr as *mut _
    }
}

pub fn get_vectortable_address() -> VirtualAddress {
    let table_addr = if SCTLR.is_set(SCTLR::VECTOR) {
        0xffff_0000
    } else {
        // We might have to check whether this register is there
        VBAR.get()
    };
    VirtualAddress::new(table_addr)
}

pub struct VectorTable {
    vectors: VectorTableMemory,
}

impl VectorTable {
    /// Creates a pointer to the vector table as set in the system registers
    ///
    /// # Safety
    /// The caller has to garantee that the address set in the SCTLR.V register or in the VBAR
    /// register points to valid memory
    pub fn new() -> Self {
        let mem = VectorTableMemory::new();
        VectorTable { vectors: mem }
    }
    pub fn init(&self, initial_address: VirtualAddress) {
        self.vectors.reset_handler.set(ASM_PC_24);
        self.vectors.undef_handler.set(ASM_PC_24);
        self.vectors.swi_handler.set(ASM_PC_24);
        self.vectors.prefetch_handler.set(ASM_PC_24);
        self.vectors.data_handler.set(ASM_PC_24);
        self.vectors.hyp_handler.set(ASM_PC_24);
        self.vectors.irq_handler.set(ASM_PC_24);
        self.vectors.fiq_handler.set(ASM_PC_24);
        self.vectors.reset_addr.set(initial_address.as_u32());
        self.vectors.undef_addr.set(initial_address.as_u32());
        self.vectors.swi_addr.set(initial_address.as_u32());
        self.vectors.prefetch_addr.set(initial_address.as_u32());
        self.vectors.data_addr.set(initial_address.as_u32());
        self.vectors.hyp_addr.set(initial_address.as_u32());
        self.vectors.irq_addr.set(initial_address.as_u32());
        self.vectors.fiq_addr.set(initial_address.as_u32());
    }
}

const ASM_PC_24: u32 = u32::swap_bytes(0x18f0_9fe5);

