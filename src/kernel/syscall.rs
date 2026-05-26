use core::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use core::arch::global_asm;

global_asm!(
    r#"
    .global linux_syscall_entry
linux_syscall_entry:
    mov r10, rdi
    mov r11, rsi
    mov r8, rdx

    mov rdi, rax
    mov rsi, r10
    mov rdx, r11
    mov rcx, r8
    xor r8, r8
    xor r9, r9

    sub rsp, 8
    mov qword ptr [rsp], 0
    call linux_syscall
    add rsp, 8
    ret
"#
);

unsafe extern "C" {
    pub fn linux_syscall_entry() -> isize;
}

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
    match num {
        SYS_READ => sys_read(a1, a2 as *mut u8, a3),
        SYS_WRITE => sys_write(a1, a2 as *const u8, a3),
        SYS_OPEN => ENOSYS,
        SYS_CLOSE => ENOSYS,
        SYS_GETPID => 1,
        SYS_EXIT => {
            EXIT_CODE.store(a1 as isize, Ordering::SeqCst);
            EXITED.store(true, Ordering::SeqCst);
            0
        }
        _ => ENOSYS,
    }
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
