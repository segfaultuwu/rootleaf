use crate::syscall::{syscall6, SYS_EXIT};

pub fn exit(code: isize) -> ! {
    syscall6(SYS_EXIT, code as usize, 0, 0, 0, 0, 0);

    loop {
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}