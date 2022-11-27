//! Miscellaneous assembly instructions

use core::arch::asm;
use core::sync::atomic::{compiler_fence, Ordering};

/// Puts the processor in Debug state. Debuggers can pick this up as a "breakpoint".
///
/// **NOTE** calling `bkpt` when the processor is not connected to a debugger will cause an
/// exception.
#[inline(always)]
pub fn bkpt() {
    unsafe { asm!("bkpt", options(nomem, nostack, preserves_flags)) };
}

/// A no-operation. Useful to prevent delay loops from being optimized away.
#[inline(always)]
pub fn nop() {
    // NOTE: This is a `pure` asm block, but applying that option allows the compiler to eliminate
    // the nop entirely (or to collapse multiple subsequent ones). Since the user probably wants N
    // nops when they call `nop` N times, let's not add that option.
    unsafe {
        asm!("nop", options(nomem, nostack, preserves_flags))
    };
}

/// Generate an Undefined Instruction exception.
///
/// Can be used as a stable alternative to `core::intrinsics::abort`.
#[inline(always)]
pub fn udf() -> ! {
    unsafe { asm!("udf #0", options(noreturn, nomem, nostack, preserves_flags)) }
}

/// Wait For Event
#[inline(always)]
pub fn wfe() {
    unsafe { asm!("wfe", options(nomem, nostack, preserves_flags)) };
}

/// Wait For Interrupt
#[inline(always)]
pub fn wfi() {
    unsafe { asm!("wfi", options(nomem, nostack, preserves_flags)) };
}

/// Send Event
#[inline(always)]
pub fn sev() {
    unsafe { asm!("sev", options(nomem, nostack, preserves_flags)) };
}

/// Instruction Synchronization Barrier
///
/// Flushes the pipeline in the processor, so that all instructions following the `ISB` are fetched
/// from cache or memory, after the instruction has been completed.
#[inline(always)]
pub fn isb() {
    compiler_fence(Ordering::SeqCst);
    unsafe {
        asm!("isb", options(nomem, nostack, preserves_flags))
    };
    compiler_fence(Ordering::SeqCst);
}

/// Data Synchronization Barrier
///
/// Acts as a special kind of memory barrier. No instruction in program order after this instruction
/// can execute until this instruction completes. This instruction completes only when both:
///
///  * any explicit memory access made before this instruction is complete
///  * all cache and branch predictor maintenance operations before this instruction complete
#[inline(always)]
pub fn dsb() {
    compiler_fence(Ordering::SeqCst);
    unsafe {
        asm!("dsb", options(nomem, nostack, preserves_flags))
    };
    compiler_fence(Ordering::SeqCst);
}

/// Data Memory Barrier
///
/// Ensures that all explicit memory accesses that appear in program order before the `DMB`
/// instruction are observed before any explicit memory accesses that appear in program order
/// after the `DMB` instruction.
#[inline(always)]
pub fn dmb() {
    compiler_fence(Ordering::SeqCst);
    unsafe {
        asm!("dmb", options(nomem, nostack, preserves_flags))
    };
    compiler_fence(Ordering::SeqCst);
}
