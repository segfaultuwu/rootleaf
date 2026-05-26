#![no_std]
#![no_main]

use rootleaf_libc::{exit, write};

#[unsafe(no_mangle)]
pub extern "C" fn _start(syscall_ptr: usize) -> ! {
    unsafe {
        rootleaf_libc::syscall::init(syscall_ptr);
    }

    write(1, b"[init] Rusty programs work somehow\n");

    exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(101);
}