use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::arch::x86_64::port::{inb, io_wait, outb};
use crate::kernel::input;

const KEYBOARD_DATA_PORT: u16 = 0x60;
const KEYBOARD_STATUS_PORT: u16 = 0x64;

static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
static CAPS_LOCK: AtomicBool = AtomicBool::new(false);
static EXTENDED_SCANCODE: AtomicBool = AtomicBool::new(false);
static IRQ_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn init() {
    unsafe fn wait_for_write() {
        for _ in 0..1_000_000 {
            if inb(KEYBOARD_STATUS_PORT) & 2 == 0 {
                break;
            }
            io_wait();
        }
    }

    unsafe fn wait_for_read() {
        for _ in 0..1_000_000 {
            if inb(KEYBOARD_STATUS_PORT) & 1 != 0 {
                break;
            }
            io_wait();
        }
    }

    unsafe {
        // 1. Disable first PS/2 port while configuring
        wait_for_write();
        outb(KEYBOARD_STATUS_PORT, 0xAD);

        // 2. Flush output buffer
        while inb(KEYBOARD_STATUS_PORT) & 1 != 0 {
            inb(KEYBOARD_DATA_PORT);
        }

        // 3. Read configuration byte
        wait_for_write();
        outb(KEYBOARD_STATUS_PORT, 0x20);
        wait_for_read();
        let mut config = inb(KEYBOARD_DATA_PORT);

        // 4. Enable first PS/2 port interrupts
        config |= 1;

        // 5. Write configuration byte
        wait_for_write();
        outb(KEYBOARD_STATUS_PORT, 0x60);
        wait_for_write();
        outb(KEYBOARD_DATA_PORT, config);

        // 6. Enable first PS/2 port
        wait_for_write();
        outb(KEYBOARD_STATUS_PORT, 0xAE);
    }
}

pub fn handle_interrupt() {
    let scancode = unsafe { inb(KEYBOARD_DATA_PORT) };
    handle_scancode(scancode);
}

/// Poll keyboard controller once and process a scancode if available.
pub fn poll_once() {
    let status = unsafe { inb(KEYBOARD_STATUS_PORT) };
    if status & 1 == 0 {
        return;
    }

    let scancode = unsafe { inb(KEYBOARD_DATA_PORT) };
    handle_scancode(scancode);
}

fn handle_scancode(scancode: u8) {
    if scancode == 0xE0 {
        EXTENDED_SCANCODE.store(true, Ordering::SeqCst);
        return;
    }

    if EXTENDED_SCANCODE.swap(false, Ordering::SeqCst) {
        handle_extended_scancode(scancode);
        return;
    }

    match scancode {
        // Left shift press / right shift press
        0x2A | 0x36 => {
            SHIFT_PRESSED.store(true, Ordering::SeqCst);
            return;
        }

        // Left shift release / right shift release
        0xAA | 0xB6 => {
            SHIFT_PRESSED.store(false, Ordering::SeqCst);
            return;
        }

        // Caps Lock press (release 0xBA handled by generic key-release check below)
        0x3A => {
            let old = CAPS_LOCK.load(Ordering::SeqCst);
            CAPS_LOCK.store(!old, Ordering::SeqCst);
            return;
        }

        _ => {}
    }

    // Key release: highest bit set
    if scancode & 0x80 != 0 {
        return;
    }

    let shift = SHIFT_PRESSED.load(Ordering::SeqCst);
    let caps = CAPS_LOCK.load(Ordering::SeqCst);

    if let Some(byte) = scancode_to_ascii(scancode, shift, caps) {
        let enq = input::enqueue(byte);
        if enq {
            IRQ_COUNTER.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn handle_extended_scancode(scancode: u8) {
    // Extended key release
    if scancode & 0x80 != 0 {
        return;
    }

    let byte = match scancode {
        0x48 => b'w', // up
        0x50 => b's', // down
        0x4B => b'a', // left
        0x4D => b'd', // right

        _ => return,
    };

    let _ = input::enqueue(byte);
    IRQ_COUNTER.fetch_add(1, Ordering::SeqCst);
}

pub fn take_irq_count() -> usize {
    IRQ_COUNTER.swap(0, Ordering::SeqCst)
}

fn scancode_to_ascii(scancode: u8, shift: bool, caps: bool) -> Option<u8> {
    let ch = match scancode {
        0x01 => 0x1B, // Escape

        0x02 => {
            if shift {
                b'!'
            } else {
                b'1'
            }
        }
        0x03 => {
            if shift {
                b'@'
            } else {
                b'2'
            }
        }
        0x04 => {
            if shift {
                b'#'
            } else {
                b'3'
            }
        }
        0x05 => {
            if shift {
                b'$'
            } else {
                b'4'
            }
        }
        0x06 => {
            if shift {
                b'%'
            } else {
                b'5'
            }
        }
        0x07 => {
            if shift {
                b'^'
            } else {
                b'6'
            }
        }
        0x08 => {
            if shift {
                b'&'
            } else {
                b'7'
            }
        }
        0x09 => {
            if shift {
                b'*'
            } else {
                b'8'
            }
        }
        0x0A => {
            if shift {
                b'('
            } else {
                b'9'
            }
        }
        0x0B => {
            if shift {
                b')'
            } else {
                b'0'
            }
        }

        0x0C => {
            if shift {
                b'_'
            } else {
                b'-'
            }
        }
        0x0D => {
            if shift {
                b'+'
            } else {
                b'='
            }
        }
        0x0E => b'\x08', // Backspace
        0x0F => b'\t',

        0x10 => letter(b'q', shift, caps),
        0x11 => letter(b'w', shift, caps),
        0x12 => letter(b'e', shift, caps),
        0x13 => letter(b'r', shift, caps),
        0x14 => letter(b't', shift, caps),
        0x15 => letter(b'y', shift, caps),
        0x16 => letter(b'u', shift, caps),
        0x17 => letter(b'i', shift, caps),
        0x18 => letter(b'o', shift, caps),
        0x19 => letter(b'p', shift, caps),

        0x1A => {
            if shift {
                b'{'
            } else {
                b'['
            }
        }
        0x1B => {
            if shift {
                b'}'
            } else {
                b']'
            }
        }
        0x1C => b'\n',

        0x1E => letter(b'a', shift, caps),
        0x1F => letter(b's', shift, caps),
        0x20 => letter(b'd', shift, caps),
        0x21 => letter(b'f', shift, caps),
        0x22 => letter(b'g', shift, caps),
        0x23 => letter(b'h', shift, caps),
        0x24 => letter(b'j', shift, caps),
        0x25 => letter(b'k', shift, caps),
        0x26 => letter(b'l', shift, caps),

        0x27 => {
            if shift {
                b':'
            } else {
                b';'
            }
        }
        0x28 => {
            if shift {
                b'"'
            } else {
                b'\''
            }
        }
        0x29 => {
            if shift {
                b'~'
            } else {
                b'`'
            }
        }

        0x2B => {
            if shift {
                b'|'
            } else {
                b'\\'
            }
        }

        0x2C => letter(b'z', shift, caps),
        0x2D => letter(b'x', shift, caps),
        0x2E => letter(b'c', shift, caps),
        0x2F => letter(b'v', shift, caps),
        0x30 => letter(b'b', shift, caps),
        0x31 => letter(b'n', shift, caps),
        0x32 => letter(b'm', shift, caps),

        0x33 => {
            if shift {
                b'<'
            } else {
                b','
            }
        }
        0x34 => {
            if shift {
                b'>'
            } else {
                b'.'
            }
        }
        0x35 => {
            if shift {
                b'?'
            } else {
                b'/'
            }
        }

        0x39 => b' ',
        0x3B => crate::kernel::input::KEY_F1, // F1
        0x3C => crate::kernel::input::KEY_F2, // F2
        0x3D => crate::kernel::input::KEY_F3, // F3
        _ => return None,
    };

    Some(ch)
}

fn letter(ch: u8, shift: bool, caps: bool) -> u8 {
    if shift ^ caps {
        ch - 32
    } else {
        ch
    }
}
