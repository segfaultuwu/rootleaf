use core::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_GETPID: usize = 39;
pub const SYS_EXIT: usize = 60;

const ENOSYS: isize = -38;
const EINVAL: isize = -22;

static EXITED: AtomicBool = AtomicBool::new(false);
static EXIT_CODE: AtomicIsize = AtomicIsize::new(0);

pub fn reset_process_state() {
    EXITED.store(false, Ordering::SeqCst);
    EXIT_CODE.store(0, Ordering::SeqCst);
}

pub fn take_exit_code() -> Option<isize> {
    if EXITED.load(Ordering::SeqCst) {
        Some(EXIT_CODE.load(Ordering::SeqCst))
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn linux_syscall(
    num: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    _a4: usize,
    _a5: usize,
    _a6: usize,
) -> isize {
    crate::drivers::serial::write_str("ELF: linux_syscall entry\n");
    match num {
        SYS_READ => {
            crate::drivers::serial::write_str("ELF: syscall READ\n");
            sys_read(a1, a2 as *mut u8, a3)
        }
        SYS_WRITE => {
            crate::drivers::serial::write_str("ELF: syscall WRITE\n");
            sys_write(a1, a2 as *const u8, a3)
        }
        SYS_OPEN => ENOSYS,
        SYS_CLOSE => ENOSYS,
        SYS_GETPID => 1,
        SYS_EXIT => {
            crate::drivers::serial::write_str("ELF: syscall EXIT\n");
            sys_exit(a1 as isize)
        }
        _ => ENOSYS,
    }
}

fn sys_exit(code: isize) -> ! {
    EXIT_CODE.store(code, Ordering::SeqCst);
    EXITED.store(true, Ordering::SeqCst);

    crate::drivers::serial::write_str("ELF: process exited with code ");
    crate::drivers::serial::write_hex(code as usize);
    crate::drivers::serial::write_str("\n");

    crate::scheduler::exit_current_task();
}

fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    if (fd != 1 && fd != 2) || buf.is_null() {
        return EINVAL;
    }

    let s = unsafe { core::slice::from_raw_parts(buf, len) };
    for &b in s {
        crate::kernel::write_byte(b);
    }
    len as isize
}

fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    if fd != 0 || buf.is_null() {
        return EINVAL;
    }

    let out = unsafe { core::slice::from_raw_parts_mut(buf, len) };
    let mut read = 0usize;

    while read < len {
        let b = loop {
            if let Some(byte) = crate::kernel::input::dequeue() {
                break byte;
            }

            crate::drivers::keyboard::poll_once();
            crate::kernel::present();
            unsafe {
                core::arch::asm!("pause", options(nomem, nostack));
            }
        };

        out[read] = b;
        read += 1;

        if b == b'\n' {
            break;
        }
    }

    read as isize
}
