#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod arch;
mod boot;
mod drivers;
mod fs;
mod kernel;
mod lib;

#[macro_use]
mod macros;

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

use crate::boot::limine::{BASE_REVISION, FRAMEBUFFER_REQUEST};
use crate::drivers::graphics::{ConsoleDriver, Psf2};
use crate::kernel::{hlt_loop, init_console};
use crate::lib::u32_to_str;

static PSF_FONT: &[u8] = include_bytes!("../assets/ter-u16n.psf");
static mut CURRENT_PATH: &str = "0:\\";

struct ConsoleStorage(UnsafeCell<MaybeUninit<ConsoleDriver>>);

unsafe impl Sync for ConsoleStorage {}

static CONSOLE_STORAGE: ConsoleStorage = ConsoleStorage(UnsafeCell::new(MaybeUninit::uninit()));

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    drivers::serial::init();
    drivers::serial::write_str("Rootleaf: serial initialized\n");

    if !BASE_REVISION.is_supported() {
        drivers::serial::write_str("Rootleaf: unsupported Limine base revision\n");
        hlt_loop();
    }

    let font = match Psf2::new(PSF_FONT) {
        Some(font) => font,
        None => {
            drivers::serial::write_str("Rootleaf: failed to load PSF2 font\n");
            hlt_loop();
        }
    };

    drivers::serial::write_str("Rootleaf: PSF font loaded, glyphs: ");

    let mut num_buf = [0u8; 10];
    let glyph_count = u32_to_str(font.char_count() as u32, &mut num_buf);

    drivers::serial::write_str(glyph_count);
    drivers::serial::write_str("\n");

    let framebuffer_response = match FRAMEBUFFER_REQUEST.response() {
        Some(response) => response,
        None => {
            drivers::serial::write_str("Rootleaf: no framebuffer response\n");
            hlt_loop();
        }
    };

    let framebuffer = match framebuffer_response.framebuffers().first() {
        Some(framebuffer) => framebuffer,
        None => {
            drivers::serial::write_str("Rootleaf: no framebuffer found\n");
            hlt_loop();
        }
    };

    drivers::serial::write_str("Rootleaf: framebuffer found\n");

    print_framebuffer_info(framebuffer);

    let fb_size = framebuffer.pitch as usize * framebuffer.height as usize;

    let fb_slice: &'static mut [u8] =
        unsafe { core::slice::from_raw_parts_mut(framebuffer.address() as *mut u8, fb_size) };

    let console: &'static mut ConsoleDriver = unsafe {
        (*CONSOLE_STORAGE.0.get()).write(ConsoleDriver::new(
            font,
            fb_slice,
            framebuffer.width as usize,
            framebuffer.height as usize,
            framebuffer.pitch as usize,
            (framebuffer.bpp / 8) as usize,
        ))
    };

    console.init();

    init_console(console);

    drivers::serial::write_str("Rootleaf: framebuffer console initialized\n");

    arch::x86_64::init();
    drivers::keyboard::init();

    let mut line = [0u8; 128];
    let mut line_len = 0usize;

    loop {
        if let Some(byte) = crate::kernel::input::dequeue() {
            handle_input_byte(byte, &mut line, &mut line_len);
            continue;
        }

        let irq_count = crate::drivers::keyboard::take_irq_count();
        if irq_count > 0 {
            continue;
        }

        crate::drivers::keyboard::poll_once();
        crate::kernel::tick_cursor();

        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}

fn handle_input_byte(byte: u8, line: &mut [u8; 128], line_len: &mut usize) {
    match byte {
        crate::kernel::input::KEY_F1 => {
            show_help_tab();
        }

        crate::kernel::input::KEY_F2 => {
            show_mem_tab();
        }

        crate::kernel::input::KEY_F3 => {
            show_disks_tab();
        }

        crate::kernel::input::KEY_ESC => {
            crate::kernel::clear_console();
        }

        b'\n' | b'\r' => {
            crate::print!("\n");

            let command = &line[..*line_len];
            execute_command(command);

            *line_len = 0;
            clear_line_buffer(line);

            crate::kernel::prompt();
        }

        b'\x08' => {
            if *line_len > 0 {
                *line_len -= 1;
                line[*line_len] = 0;
                crate::print!("\x08 \x08");
            }
        }

        byte => {
            if byte < 0x20 || byte > 0x7e {
                return;
            }

            if *line_len >= line.len() - 1 {
                return;
            }

            line[*line_len] = byte;
            *line_len += 1;

            crate::kernel::write_byte(byte);
        }
    }
}

fn execute_command(command: &[u8]) {
    let command = trim_ascii(command);

    if command.is_empty() {
        return;
    }

    if eq_ignore_ascii_case(command, b"HELP") {
        crate::print!("Available commands:\n");
        crate::print!("  HELP     Show this help\n");
        crate::print!("  VER      Show system version\n");
        crate::print!("  CLS      Clear screen\n");
        crate::print!("  CLEAR    Clear screen\n");
        crate::print!("  ECHO     Print text\n");
        crate::print!("  ABOUT    About Rootleaf\n");
        crate::print!("  PWD      Print current working directory\n");
        crate::print!("  MEM      Print memory info\n");
        crate::print!("  SYSINFO  Print OS info\n");
        crate::print!("  MMAP     Show Limine memory map\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"SYSINFO") {
        let mem = crate::kernel::memory::memory_info();

        crate::print!("Rootleaf System Information\n");
        crate::print!("Kernel:      Rootleaf v{}\n", env!("CARGO_PKG_VERSION"));
        crate::print!("Bootloader:  Limine\n");
        crate::print!("Video:       Framebuffer\n");
        crate::print!("Keyboard:    PS/2\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("RAM:         ");
        crate::kernel::write_raw(crate::lib::u64_to_str(mem.usable_mib(), &mut buf));
        crate::print!(" MiB usable\n");

        return;
    }

    if eq_ignore_ascii_case(command, b"MMAP") {
        crate::kernel::memory::print_memory_map();
        return;
    }

    if eq_ignore_ascii_case(command, b"MEM") {
        let mem = crate::kernel::memory::memory_info();

        crate::print!("Memory information:\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  RAM total:   ");
        crate::kernel::write_raw(crate::lib::u64_to_str(mem.total_mib(), &mut buf));
        crate::print!(" MiB\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  Usable:      ");
        crate::kernel::write_raw(crate::lib::u64_to_str(mem.usable_mib(), &mut buf));
        crate::print!(" MiB\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  Reserved:    ");
        crate::kernel::write_raw(crate::lib::u64_to_str(mem.reserved_mib(), &mut buf));
        crate::print!(" MiB\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  Kernel:      ");
        crate::kernel::write_raw(crate::lib::u64_to_str(
            mem.kernel_and_modules_mib(),
            &mut buf,
        ));
        crate::print!(" MiB\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  Bootloader:  ");
        crate::kernel::write_raw(crate::lib::u64_to_str(
            mem.bootloader_reclaimable_mib(),
            &mut buf,
        ));
        crate::print!(" MiB reclaimable\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  Framebuffer: ");
        crate::kernel::write_raw(crate::lib::u64_to_str(mem.framebuffer_kib(), &mut buf));
        crate::print!(" KiB\n");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  Entries:     ");
        crate::kernel::write_raw(crate::lib::u64_to_str(mem.entry_count as u64, &mut buf));
        crate::print!("\n");

        return;
    }

    if eq_ignore_ascii_case(command, b"VER") {
        crate::print!("Rootleaf Kernel [Version 0.1.0]\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"ABOUT") {
        crate::print!("Rootleaf is a small experimental OS made by segfaultuwu\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"CLS") || eq_ignore_ascii_case(command, b"CLEAR") {
        crate::kernel::clear_console();
        return;
    }

    if eq_ignore_ascii_case(command, b"PWD") {
        unsafe { crate::print!("{}\n", CURRENT_PATH) };
        return;
    }

    if eq_ignore_ascii_case(command, b"REBOOT") {
        return;
    }

    if starts_with_ignore_ascii_case(command, b"ECHO ") {
        let text = &command[5..];

        for &b in text {
            crate::print!("{}", b as char);
        }

        crate::print!("\n");
        return;
    }

    crate::print!("Bad command or file name\n");
}

fn print_framebuffer_info(framebuffer: &limine::framebuffer::Framebuffer) {
    drivers::serial::write_str("Framebuffer info:\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("width=");
    drivers::serial::write_str(u32_to_str(framebuffer.width as u32, &mut buf));
    drivers::serial::write_str("\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("height=");
    drivers::serial::write_str(u32_to_str(framebuffer.height as u32, &mut buf));
    drivers::serial::write_str("\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("pitch=");
    drivers::serial::write_str(u32_to_str(framebuffer.pitch as u32, &mut buf));
    drivers::serial::write_str("\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("bpp=");
    drivers::serial::write_str(u32_to_str(framebuffer.bpp as u32, &mut buf));
    drivers::serial::write_str("\n");
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

fn clear_line_buffer(line: &mut [u8; 128]) {
    for byte in line.iter_mut() {
        *byte = 0;
    }
}

fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for i in 0..a.len() {
        if to_ascii_upper(a[i]) != to_ascii_upper(b[i]) {
            return false;
        }
    }

    true
}

fn starts_with_ignore_ascii_case(a: &[u8], prefix: &[u8]) -> bool {
    if a.len() < prefix.len() {
        return false;
    }

    eq_ignore_ascii_case(&a[..prefix.len()], prefix)
}

fn to_ascii_upper(byte: u8) -> u8 {
    if byte >= b'a' && byte <= b'z' {
        byte - 32
    } else {
        byte
    }
}

fn show_help_tab() {
    crate::kernel::clear_console();

    crate::print!("Rootleaf Help\n");
    crate::print!("=============\n\n");

    crate::print!("Keyboard shortcuts:\n");
    crate::print!("  F1      Help tab\n");
    crate::print!("  F2      Memory tab\n");
    crate::print!("  F3      Disks tab\n");
    crate::print!("  ESC     Return to shell\n\n");

    crate::print!("Commands:\n");
    crate::print!("  HELP    Show help\n");
    crate::print!("  VER     Show system version\n");
    crate::print!("  MEM     Show memory information\n");
    crate::print!("  MMAP    Show memory map\n");
    crate::print!("  CLS     Clear screen\n");
    crate::print!("  CLEAR   Clear screen\n");
    crate::print!("  ECHO    Print text\n");
    crate::print!("  PWD     Print current directory\n");
    crate::print!("  REBOOT  Restart machine\n\n");

    crate::kernel::prompt();
}

fn show_mem_tab() {
    crate::kernel::clear_console();

    let mem = crate::kernel::memory::memory_info();

    crate::print!("Memory\n");
    crate::print!("======\n\n");

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("RAM total:   ");
    crate::kernel::write_raw(crate::lib::u64_to_str(mem.total_mib(), &mut buf));
    crate::print!(" MiB\n");

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("Usable:      ");
    crate::kernel::write_raw(crate::lib::u64_to_str(mem.usable_mib(), &mut buf));
    crate::print!(" MiB\n");

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("Reserved:    ");
    crate::kernel::write_raw(crate::lib::u64_to_str(mem.reserved_mib(), &mut buf));
    crate::print!(" MiB\n");

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("Framebuffer: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(mem.framebuffer_kib(), &mut buf));
    crate::print!(" KiB\n");

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("Entries:     ");
    crate::kernel::write_raw(crate::lib::u64_to_str(mem.entry_count as u64, &mut buf));
    crate::print!("\n\n");

    crate::kernel::prompt();
}

fn show_disks_tab() {
    crate::kernel::clear_console();

    crate::print!("Disks\n");
    crate::print!("=====\n\n");

    crate::print!("Disk support is not implemented yet.\n\n");
    crate::print!("Planned:\n");
    crate::print!("  - AHCI/SATA detection\n");
    crate::print!("  - NVMe detection\n");
    crate::print!("  - FAT32 reader\n");
    crate::print!("  - DIR command from real filesystem\n\n");

    crate::kernel::prompt();
}
