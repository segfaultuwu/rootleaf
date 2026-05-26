use crate::syscall::{syscall6, SYS_READ, SYS_WRITE, SYS_CLOSE, SYS_OPEN};

pub fn write(fd: usize, buf: &[u8]) -> isize {
    syscall6(
        SYS_WRITE,
        fd,
        buf.as_ptr() as usize,
        buf.len(),
        0,
        0,
        0,
    )
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    syscall6(
        SYS_READ,
        fd,
        buf.as_mut_ptr() as usize,
        buf.len(),
        0,
        0,
        0,
    )
}

pub fn open(path: *const u8, flags: usize, mode: usize) -> isize {
    syscall6(SYS_OPEN, path as usize, flags, mode, 0, 0, 0)
}

pub fn close(fd: usize) -> isize {
    syscall6(SYS_CLOSE, fd, 0, 0, 0, 0, 0)
}