use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

pub const SYS_READ: usize = 0;
pub const SYS_WRITE: usize = 1;
pub const SYS_OPEN: usize = 2;
pub const SYS_CLOSE: usize = 3;
pub const SYS_LSEEK: usize = 8;
pub const SYS_BRK: usize = 12;
pub const SYS_GETPID: usize = 39;
pub const SYS_UNAME: usize = 63;
pub const SYS_GETCWD: usize = 79;
pub const SYS_EXIT: usize = 60;
pub const SYS_EXIT_GROUP: usize = 231;

const EPERM: isize = -1;
const ENOENT: isize = -2;
const EBADF: isize = -9;
const ENOMEM: isize = -12;
const EFAULT: isize = -14;
const EINVAL: isize = -22;
const ENOSYS: isize = -38;

const MAX_FDS: usize = 16;
const MAX_PATH: usize = 128;
const MAX_RW: usize = 4096;

const SEEK_SET: usize = 0;
const SEEK_CUR: usize = 1;
const SEEK_END: usize = 2;

static EXITED: AtomicBool = AtomicBool::new(false);
static EXIT_CODE: AtomicIsize = AtomicIsize::new(0);

#[derive(Clone, Copy)]
struct FileDesc {
    used: bool,
    data: &'static [u8],
    pos: usize,
}

const EMPTY_FD: FileDesc = FileDesc {
    used: false,
    data: &[],
    pos: 0,
};

struct FdTable(UnsafeCell<[FileDesc; MAX_FDS]>);
unsafe impl Sync for FdTable {}

static FD_TABLE: FdTable = FdTable(UnsafeCell::new([EMPTY_FD; MAX_FDS]));

// Very primitive brk state.
// Real brk needs real user heap mapping later.
static mut PROGRAM_BREAK: usize = 0;

pub fn reset_process_state() {
    EXITED.store(false, Ordering::SeqCst);
    EXIT_CODE.store(0, Ordering::SeqCst);

    unsafe {
        let fds = &mut *FD_TABLE.0.get();

        for fd in 3..MAX_FDS {
            fds[fd] = EMPTY_FD;
        }

        PROGRAM_BREAK = 0;
    }
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
    a4: usize,
    _a5: usize,
    _a6: usize,
) -> isize {
    match num {
        SYS_READ => sys_read(a1, a2 as *mut u8, a3),

        SYS_WRITE => sys_write(a1, a2 as *const u8, a3),

        SYS_OPEN => sys_open(a1 as *const u8, a2, a3),

        SYS_CLOSE => sys_close(a1),

        SYS_LSEEK => sys_lseek(a1, a2 as isize, a3),

        SYS_BRK => sys_brk(a1),

        SYS_GETPID => sys_getpid(),

        SYS_GETCWD => sys_getcwd(a1 as *mut u8, a2),

        SYS_UNAME => sys_uname(a1 as *mut u8),

        SYS_EXIT => sys_exit(a1 as isize),

        SYS_EXIT_GROUP => sys_exit(a1 as isize),

        _ => {
            crate::drivers::serial::write_str("ELF: unknown syscall ");
            crate::drivers::serial::write_hex(num);
            crate::drivers::serial::write_str("\n");
            ENOSYS
        }
    }
}

fn sys_getpid() -> isize {
    crate::scheduler::current_task_id() as isize
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
    if fd != 1 && fd != 2 {
        return EBADF;
    }

    if buf.is_null() {
        return EFAULT;
    }

    if len > MAX_RW {
        return EINVAL;
    }

    for i in 0..len {
        let b = unsafe {
            core::ptr::read_volatile(buf.add(i))
        };

        crate::kernel::write_byte(b);
    }

    crate::kernel::present();

    len as isize
}

fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    if buf.is_null() {
        return EFAULT;
    }

    if len > MAX_RW {
        return EINVAL;
    }

    if fd == 0 {
        return sys_read_stdin(buf, len);
    }

    sys_read_file(fd, buf, len)
}

fn sys_read_stdin(buf: *mut u8, len: usize) -> isize {
    let out = unsafe {
        core::slice::from_raw_parts_mut(buf, len)
    };

    let mut read = 0usize;

    while read < len {
        let b = loop {
            if let Some(byte) = crate::kernel::input::dequeue() {
                break byte;
            }

            crate::drivers::keyboard::poll_once();
            crate::kernel::present();

            crate::scheduler::yield_now();

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

fn sys_read_file(fd: usize, buf: *mut u8, len: usize) -> isize {
    unsafe {
        let fds = &mut *FD_TABLE.0.get();

        if fd >= MAX_FDS || !fds[fd].used {
            return EBADF;
        }

        let desc = &mut fds[fd];

        if desc.pos >= desc.data.len() {
            return 0;
        }

        let remaining = desc.data.len() - desc.pos;
        let count = core::cmp::min(len, remaining);

        let out = core::slice::from_raw_parts_mut(buf, count);
        let src = &desc.data[desc.pos..desc.pos + count];

        out.copy_from_slice(src);

        desc.pos += count;

        count as isize
    }
}

fn sys_open(path_ptr: *const u8, flags: usize, _mode: usize) -> isize {
    // For now: read-only only.
    // Linux O_RDONLY == 0.
    if flags != 0 {
        return EPERM;
    }

    if path_ptr.is_null() {
        return EFAULT;
    }

    let mut path_buf = [0u8; MAX_PATH];

    let path = match copy_cstr_from_user(path_ptr, &mut path_buf) {
        Some(path) => path,
        None => return EFAULT,
    };

    crate::drivers::serial::write_str("ELF: open path = '");
    crate::drivers::serial::write_str(path);
    crate::drivers::serial::write_str("'\n");

    let data = match crate::fs::vfs::read(path) {
        Ok(data) => {
            crate::drivers::serial::write_str("ELF: open ok, size=");
            crate::drivers::serial::write_hex(data.len());
            crate::drivers::serial::write_str("\n");

            data
        }

        Err(e) => {
            crate::drivers::serial::write_str("ELF: open failed: ");
            crate::drivers::serial::write_str(crate::fs::vfs::error_str(e));
            crate::drivers::serial::write_str("\n");

            return match e {
                // jeśli masz konkretne enumy, dostosuj nazwy
                _ => ENOENT,
            };
        }
    };

    unsafe {
        let fds = &mut *FD_TABLE.0.get();

        for fd in 3..MAX_FDS {
            if !fds[fd].used {
                fds[fd] = FileDesc {
                    used: true,
                    data,
                    pos: 0,
                };

                crate::drivers::serial::write_str("ELF: open fd=");
                crate::drivers::serial::write_hex(fd);
                crate::drivers::serial::write_str("\n");

                return fd as isize;
            }
        }
    }

    ENOMEM
}

fn sys_close(fd: usize) -> isize {
    if fd < 3 || fd >= MAX_FDS {
        return EBADF;
    }

    unsafe {
        let fds = &mut *FD_TABLE.0.get();

        if !fds[fd].used {
            return EBADF;
        }

        fds[fd] = EMPTY_FD;
    }

    0
}

fn sys_lseek(fd: usize, offset: isize, whence: usize) -> isize {
    unsafe {
        let fds = &mut *FD_TABLE.0.get();

        if fd >= MAX_FDS || !fds[fd].used {
            return EBADF;
        }

        let desc = &mut fds[fd];

        let base = match whence {
            SEEK_SET => 0isize,
            SEEK_CUR => desc.pos as isize,
            SEEK_END => desc.data.len() as isize,
            _ => return EINVAL,
        };

        let new_pos = base.checked_add(offset).ok_or(());

        let new_pos = match new_pos {
            Ok(v) if v >= 0 => v as usize,
            _ => return EINVAL,
        };

        desc.pos = core::cmp::min(new_pos, desc.data.len());

        desc.pos as isize
    }
}

fn sys_brk(addr: usize) -> isize {
    unsafe {
        if PROGRAM_BREAK == 0 {
            PROGRAM_BREAK = 0xffffffff80300000;
        }

        if addr == 0 {
            return PROGRAM_BREAK as isize;
        }

        // Temporary fake heap limit: 1 MiB.
        let heap_start = 0xffffffff80300000usize;
        let heap_end = heap_start + 1024 * 1024;

        if addr < heap_start || addr > heap_end {
            return PROGRAM_BREAK as isize;
        }

        PROGRAM_BREAK = addr;

        PROGRAM_BREAK as isize
    }
}

fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    if buf.is_null() {
        return EFAULT;
    }

    if size == 0 {
        return EINVAL;
    }

    let cwd = crate::fs::cwd::get().as_bytes();

    if cwd.len() + 1 > size {
        return EINVAL;
    }

    unsafe {
        for i in 0..cwd.len() {
            core::ptr::write(buf.add(i), cwd[i]);
        }

        core::ptr::write(buf.add(cwd.len()), 0);
    }

    buf as isize
}

fn sys_uname(buf: *mut u8) -> isize {
    if buf.is_null() {
        return EFAULT;
    }

    // Linux struct utsname usually has 6 fields of 65 bytes:
    // sysname, nodename, release, version, machine, domainname.
    // We fill a compatible simple layout.
    const FIELD_LEN: usize = 65;

    unsafe {
        write_uts_field(buf.add(0 * FIELD_LEN), b"Rootleaf");
        write_uts_field(buf.add(1 * FIELD_LEN), b"rootleaf");
        write_uts_field(buf.add(2 * FIELD_LEN), env!("CARGO_PKG_VERSION").as_bytes());
        write_uts_field(buf.add(3 * FIELD_LEN), b"Rootleaf Kernel");
        write_uts_field(buf.add(4 * FIELD_LEN), b"x86_64");
        write_uts_field(buf.add(5 * FIELD_LEN), b"local");
    }

    0
}

unsafe fn write_uts_field(dst: *mut u8, text: &[u8]) {
    const FIELD_LEN: usize = 65;

    let mut i = 0usize;

    while i < FIELD_LEN {
        core::ptr::write(dst.add(i), 0);
        i += 1;
    }

    let count = core::cmp::min(text.len(), FIELD_LEN - 1);

    for i in 0..count {
        core::ptr::write(dst.add(i), text[i]);
    }
}

fn copy_cstr_from_user<'a>(ptr: *const u8, out: &'a mut [u8; MAX_PATH]) -> Option<&'a str> {
    for i in 0..MAX_PATH - 1 {
        let b = unsafe {
            core::ptr::read_volatile(ptr.add(i))
        };

        if b == 0 {
            return core::str::from_utf8(&out[..i]).ok();
        }

        out[i] = b;
    }

    None
}