use core::sync::atomic::{AtomicUsize, Ordering};

const INPUT_BUF_SIZE: usize = 1024;

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
        INPUT_BUF[head] = byte;
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

    let byte = unsafe { INPUT_BUF[tail] };
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
            core::arch::asm!("sti; hlt", options(nomem, nostack));
        }
    }
}
