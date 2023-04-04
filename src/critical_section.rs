use critical_section::{set_impl, Impl, RawRestoreState};

struct SingleCoreCriticalSection;
set_impl!(SingleCoreCriticalSection);

unsafe impl Impl for SingleCoreCriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        let mut cpsr_old: u32;
        core::arch::asm!("mrs {}, cpsr", out(reg) cpsr_old);
        core::arch::asm!("cpsid i");
        cpsr_old
    }

    unsafe fn release(cpsr_old: RawRestoreState) {
        if cpsr_old & 0x80 != 0 {
            core::arch::asm!("cpsie i");
        }
    }
}
