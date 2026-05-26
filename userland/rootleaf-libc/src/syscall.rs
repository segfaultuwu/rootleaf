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
pub const SYS_EXIT: usize = 60;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_LSEEK: usize = 8;
pub const SYS_BRK: usize = 12;
pub const SYS_GETPID: usize = 39;
pub const SYS_GETCWD: usize = 79;
pub const SYS_UNAME: usize = 63;
pub const SYS_EXIT_GROUP: usize = 231;

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

pub fn lseek(fd: usize, offset: isize, whence: usize) -> isize {
    syscall6(SYS_LSEEK, fd, offset as usize, whence, 0, 0, 0)
}

pub fn getpid() -> isize {
    syscall6(SYS_GETPID, 0, 0, 0, 0, 0, 0)
}

pub fn getcwd(buf: &mut [u8]) -> isize {
    syscall6(SYS_GETCWD, buf.as_mut_ptr() as usize, buf.len(), 0, 0, 0, 0)
}

pub fn brk(addr: usize) -> isize {
    syscall6(SYS_BRK, addr, 0, 0, 0, 0, 0)
}

pub fn exit_group(code: isize) -> ! {
    syscall6(SYS_EXIT_GROUP, code as usize, 0, 0, 0, 0, 0);

    loop {
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}