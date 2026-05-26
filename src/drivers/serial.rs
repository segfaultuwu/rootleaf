use crate::arch::x86_64::port::{inb, outb};

const COM1: u16 = 0x3F8;

pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);

        outb(COM1 + 0, 0x03);
        outb(COM1 + 1, 0x00);

        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xC7);
        outb(COM1 + 4, 0x0B);
    }
}

fn is_transmit_empty() -> bool {
    unsafe { inb(COM1 + 5) & 0x20 != 0 }
}

pub fn write_byte(byte: u8) {
    while !is_transmit_empty() {}

    unsafe {
        outb(COM1, byte);
    }
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        match byte {
            b'\n' => {
                write_byte(b'\r');
                write_byte(b'\n');
            }
            byte => write_byte(byte),
        }
    }
}

pub fn write_hex(value: usize) {
    write_str("0x");

    let mut started = false;

    for i in (0..core::mem::size_of::<usize>() * 2).rev() {
        let nibble = ((value >> (i * 4)) & 0xF) as u8;

        if nibble != 0 || started || i == 0 {
            started = true;

            let ch = match nibble {
                0..=9 => b'0' + nibble,
                10..=15 => b'a' + (nibble - 10),
                _ => b'?',
            };

            write_byte(ch);
        }
    }
}
