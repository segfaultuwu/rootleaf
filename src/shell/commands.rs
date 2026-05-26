use crate::shell::path::{
    eq_ignore_ascii_case, make_absolute_path, starts_with_ignore_ascii_case, trim_ascii,
};

pub fn execute_command(command: &[u8]) {
    let command = trim_ascii(command);

    if command.is_empty() {
        return;
    }

    if eq_ignore_ascii_case(command, b"HELP") {
        help();
        return;
    }

    if eq_ignore_ascii_case(command, b"SYSINFO") {
        sysinfo();
        return;
    }

    if eq_ignore_ascii_case(command, b"MMAP") {
        crate::memory::info::print_memory_map();
        return;
    }

    if eq_ignore_ascii_case(command, b"MEM") {
        mem();
        return;
    }

    if eq_ignore_ascii_case(command, b"DIR") {
        dir();
        return;
    }

    if starts_with_ignore_ascii_case(command, b"TYPE ") {
        type_file(&command[5..]);
        return;
    }

    if eq_ignore_ascii_case(command, b"VER") {
        crate::kernel::write_raw("Rootleaf Kernel [Version ");
        crate::kernel::write_raw(env!("CARGO_PKG_VERSION"));
        crate::print!("]\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"ABOUT") {
        crate::print!("Rootleaf is a small experimental OS made by segfaultuwu\n");
        crate::print!("Source code avaiable at: https://github.com/segfaultuwu/rootleaf\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"CLS") || eq_ignore_ascii_case(command, b"CLEAR") {
        crate::kernel::clear_console();
        return;
    }

    if eq_ignore_ascii_case(command, b"PWD") {
        crate::kernel::write_raw(unsafe { crate::CURRENT_PATH });
        crate::print!("\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"REBOOT") {
        crate::print!("Rebooting...\n");
        crate::arch::x86_64::reboot();
    }

    if starts_with_ignore_ascii_case(command, b"ECHO ") {
        echo(&command[5..]);
        return;
    }

    crate::print!("Bad command or file name\n");
}

fn help() {
    crate::print!("Available commands:\n");
    crate::print!("  HELP           Show this help\n");
    crate::print!("  VER            Show system version\n");
    crate::print!("  CLS            Clear screen\n");
    crate::print!("  CLEAR          Clear screen\n");
    crate::print!("  ECHO <TEXT>    Print text\n");
    crate::print!("  ABOUT          About Rootleaf\n");
    crate::print!("  PWD            Print current working directory\n");
    crate::print!("  MEM            Print memory info\n");
    crate::print!("  SYSINFO        Print OS info\n");
    crate::print!("  MMAP           Show Limine memory map\n");
    crate::print!("  DIR            List files\n");
    crate::print!("  TYPE <FILE>    Show the contents of a file\n");
    crate::print!("  REBOOT         Restart machine\n");
}

fn sysinfo() {
    let mem = crate::memory::info::memory_info();

    crate::print!("Rootleaf System Information\n");

    crate::kernel::write_raw("Kernel:      Rootleaf v");
    crate::kernel::write_raw(env!("CARGO_PKG_VERSION"));
    crate::print!("\n");

    crate::print!("Bootloader:  Limine\n");
    crate::print!("Video:       Framebuffer\n");
    crate::print!("Keyboard:    PS/2\n");

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("RAM:         ");
    crate::kernel::write_raw(crate::lib::u64_to_str(mem.usable_mib(), &mut buf));
    crate::print!(" MiB usable\n");
}

fn mem() {
    let mem = crate::memory::info::memory_info();

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
}

fn dir() {
    match crate::fs::vfs::list(unsafe { crate::CURRENT_PATH }) {
        Ok(files) => {
            crate::kernel::write_raw(" Directory of ");
            crate::kernel::write_raw(unsafe { crate::CURRENT_PATH });
            crate::print!("\n\n");

            for file in files {
                crate::kernel::write_raw("  ");
                crate::kernel::write_raw(file.name);
                crate::print!("\n");
            }
        }

        Err(error) => {
            crate::kernel::write_raw(crate::fs::vfs::error_str(error));
            crate::print!("\n");
        }
    }
}

fn type_file(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => match crate::fs::vfs::read(path) {
            Ok(data) => {
                if let Ok(text) = core::str::from_utf8(data) {
                    crate::kernel::write_raw(text);

                    if !text.ends_with('\n') {
                        crate::print!("\n");
                    }
                } else {
                    crate::print!("Cannot display binary file\n");
                }
            }

            Err(error) => {
                crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                crate::print!("\n");
            }
        },

        None => {
            crate::print!("Invalid path\n");
        }
    }
}

fn echo(text: &[u8]) {
    for &b in text {
        crate::kernel::write_byte(b);
    }

    crate::print!("\n");
}
