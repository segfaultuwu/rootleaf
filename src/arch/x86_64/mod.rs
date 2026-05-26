pub mod idt;
pub mod pic;
pub mod port;

pub fn init() {
    unsafe {
        pic::remap();
    }

    // Debug: print PIC masks after remap
    let pic1_mask = unsafe { crate::arch::x86_64::port::inb(0x21) };
    let pic2_mask = unsafe { crate::arch::x86_64::port::inb(0xA1) };
    crate::drivers::serial::write_str("PIC masks after remap: ");
    crate::drivers::serial::write_byte(nibble_hex((pic1_mask >> 4) & 0xF));
    crate::drivers::serial::write_byte(nibble_hex(pic1_mask & 0xF));
    crate::drivers::serial::write_str(" ");
    crate::drivers::serial::write_byte(nibble_hex((pic2_mask >> 4) & 0xF));
    crate::drivers::serial::write_byte(nibble_hex(pic2_mask & 0xF));
    crate::drivers::serial::write_str("\n");

    idt::init_idt();

    unsafe {
        pic::enable_irq(1);
        enable_interrupts();
    }

    // Debug: print PIC masks after enabling IRQ1
    let pic1_mask2 = unsafe { crate::arch::x86_64::port::inb(0x21) };
    let pic2_mask2 = unsafe { crate::arch::x86_64::port::inb(0xA1) };
    crate::drivers::serial::write_str("PIC masks after enable_irq(1): ");
    crate::drivers::serial::write_byte(nibble_hex((pic1_mask2 >> 4) & 0xF));
    crate::drivers::serial::write_byte(nibble_hex(pic1_mask2 & 0xF));
    crate::drivers::serial::write_str(" ");
    crate::drivers::serial::write_byte(nibble_hex((pic2_mask2 >> 4) & 0xF));
    crate::drivers::serial::write_byte(nibble_hex(pic2_mask2 & 0xF));
    crate::drivers::serial::write_str("\n");
}

fn nibble_hex(n: u8) -> u8 {
    match n & 0xF {
        v @ 0..=9 => b'0' + v,
        v => b'a' + (v - 10),
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