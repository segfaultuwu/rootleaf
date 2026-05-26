pub mod idt;
pub mod pic;
pub mod port;

pub fn init() {
    unsafe {
        pic::remap();
    }

    idt::init_idt();

    unsafe {
        pic::enable_irq(1);
        enable_interrupts();
    }
}

pub unsafe fn enable_interrupts() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }
}

pub unsafe fn disable_interrupts() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}