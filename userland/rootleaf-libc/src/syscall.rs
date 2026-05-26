type SyscallFn = extern "C" fn(
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
) -> isize;

static mut SYSCALL_PTR: usize = 0;

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_GETPID: usize = 39;
pub const SYS_EXIT: usize = 60;

pub unsafe fn init(ptr: usize) {
    SYSCALL_PTR = ptr;
}

pub fn syscall6(
    num: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
) -> isize {
    let ptr = unsafe { SYSCALL_PTR };

    if ptr == 0 {
        return -38;
    }

    let f: SyscallFn = unsafe {
        core::mem::transmute(ptr)
    };

    f(num, a1, a2, a3, a4, a5, a6)
}