use core::sync::atomic::{AtomicUsize, Ordering};

const INPUT_BUF_SIZE: usize = 1024;

pub const KEY_F1: u8 = 0xF1;
pub const KEY_F2: u8 = 0xF2;
pub const KEY_F3: u8 = 0xF3;
pub const KEY_ESC: u8 = 0x1B;

static mut INPUT_BUF: [u8; INPUT_BUF_SIZE] = [0; INPUT_BUF_SIZE];
static HEAD: AtomicUsize = AtomicUsize::new(0);
static TAIL: AtomicUsize = AtomicUsize::new(0);

pub fn enqueue(byte: u8) -> bool {
    let head = HEAD.load(Ordering::Relaxed);
    let tail = TAIL.load(Ordering::Acquire);

    let next = (head + 1) % INPUT_BUF_SIZE;

    if next == tail {
        // buffer full
        return false;
    }

    unsafe {
        let base = core::ptr::addr_of_mut!(INPUT_BUF) as *mut u8;
        core::ptr::write_volatile(base.add(head), byte);
    }

    HEAD.store(next, Ordering::Release);
    true
}

pub fn dequeue() -> Option<u8> {
    let tail = TAIL.load(Ordering::Relaxed);
    let head = HEAD.load(Ordering::Acquire);

    if tail == head {
        return None;
    }

    let byte = unsafe {
        let base = core::ptr::addr_of_mut!(INPUT_BUF) as *mut u8;
        core::ptr::read_volatile(base.add(tail))
    };
    let next = (tail + 1) % INPUT_BUF_SIZE;
    TAIL.store(next, Ordering::Release);
    Some(byte)
}

/// Block until a byte is available and return it.
pub fn wait_for_byte() -> u8 {
    loop {
        if let Some(b) = dequeue() {
            return b;
        }

        // Enable interrupts and halt until the next interrupt wakes us.
        unsafe {
            core::arch::asm!("sti; hlt", options(nostack));
        }
    }
}
