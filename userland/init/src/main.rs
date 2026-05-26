#![no_std]
#![no_main]

use rootleaf_libc::{close, exit, open, read, write};

fn write_num(mut n: isize) {
    let mut buf = [0u8; 32];
    let mut i = buf.len();

    if n == 0 {
        write(1, b"0");
        return;
    }

    if n < 0 {
        write(1, b"-");
        n = -n;
    }

    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }

    write(1, &buf[i..]);
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(syscall_ptr: usize) -> ! {
    unsafe {
        rootleaf_libc::syscall::init(syscall_ptr);
    }

    write(1, b"[init] started\n");
    // IMPORTANT: sys_open expects a C string.
    let path = b"/disk1/README.TXT\0";

    let fd = open(path.as_ptr(), 0, 0);

    if fd < 0 {
        write(1, b"[init] open failed: ");
        write_num(fd);
        write(1, b"\n");
        exit(1);
    }

    write(1, b"[init] open ok");

    let mut buf = [0u8; 128];

    let n = read(fd as usize, &mut buf);

    if n < 0 {
        write(1, b"[init] read failed: ");
        // write_num(n);
        write(1, b"\n");
        close(fd as usize);
        exit(2);
    }

    write(1, b"[init] file contents:\n");
    write(1, &buf[..n as usize]);

    if n == 0 || buf[(n as usize).saturating_sub(1)] != b'\n' {
        write(1, b"\n");
    }

    close(fd as usize);

    write(1, b"[init] Rusty programs work somehow\n");

    exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(101);
}