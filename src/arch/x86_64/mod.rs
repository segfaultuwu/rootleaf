pub mod idt;
pub mod pic;
pub mod port;
pub mod cpu;

pub fn init() {
    unsafe {
        pic::remap();
    }

    cpu::init();

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
        const USE_KEYBOARD_IRQ: bool = false;

        if USE_KEYBOARD_IRQ {
            pic::enable_irq(1);
        } else {
            crate::drivers::serial::write_str("[kbd] IRQ1 disabled, using polling mode\n");
        }
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

use self::port::{inb, outb};

const KEYBOARD_CONTROLLER_STATUS_PORT: u16 = 0x64;
const KEYBOARD_CONTROLLER_COMMAND_PORT: u16 = 0x64;
const KEYBOARD_CONTROLLER_RESET_COMMAND: u8 = 0xFE;

pub fn reboot() -> ! {
    unsafe {
        // Poczekaj aż input buffer kontrolera 8042 będzie pusty.
        while inb(KEYBOARD_CONTROLLER_STATUS_PORT) & 0x02 != 0 {}

        // Komenda 0xFE prosi kontroler klawiatury o reset CPU.
        outb(
            KEYBOARD_CONTROLLER_COMMAND_PORT,
            KEYBOARD_CONTROLLER_RESET_COMMAND,
        );
    }

    // Fallback, jeśli reset przez 8042 nie zadziała.
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
