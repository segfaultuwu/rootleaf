use crate::syscall::{syscall6, SYS_READ, SYS_WRITE};

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