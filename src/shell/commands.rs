// Please don't hate on this file.. It's just a big match statement for commands.
// I don't want to split it into multiple files for now since it's easier to maintain as one file,
// and it's not even that big. (yes, vscode lags when i open it, but it's bearable)
// (this is temporary until we have full cross compiler)
use crate::fs::vfs::VfsBackend;
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

    if eq_ignore_ascii_case(command, b"DIR") || eq_ignore_ascii_case(command, b"LS") {
        dir_current();
        return;
    }

    if starts_with_ignore_ascii_case(command, b"DIR ") {
        dir_path(&command[4..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"LS ") {
        dir_path(&command[3..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"TYPE ") {
        type_file(&command[5..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"CAT ") {
        type_file(&command[4..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"HD ") {
        hexdump_file(&command[3..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"HEXDUMP ") {
        hexdump_file(&command[8..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"MOUNT ") {
        mount_cmd(&command[6..]);
        return;
    }

    if eq_ignore_ascii_case(command, b"UMOUNT") || eq_ignore_ascii_case(command, b"UNMOUNT") {
        umount_cmd();
        return;
    }

    if eq_ignore_ascii_case(command, b"DISKINFO") {
        diskinfo();
        return;
    }

    if eq_ignore_ascii_case(command, b"MOUNTS") {
        mounts();
        return;
    }

    if eq_ignore_ascii_case(command, b"LSDEV") {
        lsdev();
        return;
    }
    if eq_ignore_ascii_case(command, b"LSBLK") {
        lsblk();
        return;
    }

    if eq_ignore_ascii_case(command, b"TASKS") {
        tasks();
        return;
    }

    if eq_ignore_ascii_case(command, b"VER") || eq_ignore_ascii_case(command, b"VERSION") {
        crate::kernel::write_raw("Rootleaf Kernel [Version ");
        crate::kernel::write_raw(env!("CARGO_PKG_VERSION"));
        crate::print!("]\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"ABOUT") {
        crate::print!("Rootleaf is a small experimental OS made by segfaultuwu\n");
        crate::print!("Source code available at: https://github.com/segfaultuwu/rootleaf\n");
        return;
    }

    if eq_ignore_ascii_case(command, b"CLS") || eq_ignore_ascii_case(command, b"CLEAR") {
        crate::kernel::clear_console();
        return;
    }

    if eq_ignore_ascii_case(command, b"PWD") {
        crate::kernel::write_raw(crate::fs::cwd::get());
        crate::print!("\n");
        return;
    }

    if starts_with_ignore_ascii_case(command, b"CD ") || eq_ignore_ascii_case(command, b"CD") {
        let arg = if command.len() > 2 {
            &command[2..]
        } else {
            b""
        };
        cd_cmd(arg);
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

    if starts_with_ignore_ascii_case(command, b"TOUCH ") {
        touch_cmd(&command[6..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"WRITE ") {
        write_cmd(&command[6..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"EDIT ") {
        edit_cmd(&command[5..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"RUN ") {
        run_script(&command[4..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"ELF ") {
        elf_cmd(&command[4..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"EXEC ") {
        elf_cmd(&command[5..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"RM ") {
        rm_cmd(&command[3..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"RMD ") {
        rm_cmd(&command[4..]);
        return;
    }

    if starts_with_ignore_ascii_case(command, b"DEL ") {
        rm_cmd(&command[4..]);
        return;
    }

    // If the user typed a path-like token (./app.elf, /disk1/app.elf, etc.), try executing it.
    if command.starts_with(b"./") || command.starts_with(b"/") || command.iter().any(|&c| c == b'/')
    {
        elf_cmd(command);
        return;
    }

    crate::print!("Bad command or file name\n");
}

fn lsblk() {
    crate::print!("NAME     PART   MOUNTPOINT\n");

    crate::print!("loop0\n");
    crate::print!("  - loop0p1   /ram\n");

    let sata_count = crate::drivers::sata::drive_count();
    let ata = crate::drivers::pci::scan_legacy_ata();
    let ata_count = ata.ata_devices as usize;

    let total = sata_count + ata_count;

    for i in 0..total {
        let name = sd_name(i);

        crate::kernel::write_raw(name);
        crate::print!("\n");

        crate::kernel::write_raw("  - ");
        crate::kernel::write_raw(name);
        crate::kernel::write_raw("1   ");

        if i == 0 && crate::fs::fat32::is_mounted() {
            crate::kernel::write_raw("/disk1");
        } else {
            crate::kernel::write_raw("(not mounted)");
        }

        crate::print!("\n");
    }

    after_vfs_op();
}

fn sd_name(index: usize) -> &'static str {
    match index {
        0 => "sda",
        1 => "sdb",
        2 => "sdc",
        3 => "sdd",
        4 => "sde",
        5 => "sdf",
        6 => "sdg",
        7 => "sdh",
        8 => "sdi",
        9 => "sdj",
        10 => "sdk",
        11 => "sdl",
        12 => "sdm",
        13 => "sdn",
        14 => "sdo",
        15 => "sdp",
        _ => "sd?",
    }
}

fn pad_name(name: &str, width: usize) {
    let len = name.len();

    if len >= width {
        return;
    }

    for _ in 0..(width - len) {
        crate::kernel::write_byte(b' ');
    }
}

fn pad_name_with_extra(name: &str, width: usize, extra: usize) {
    let len = name.len() + extra;

    if len >= width {
        return;
    }

    for _ in 0..(width - len) {
        crate::kernel::write_byte(b' ');
    }
}

fn help() {
    crate::print!("Available commands:\n");
    crate::print!("  HELP                 Show this help\n");
    crate::print!("  VER, VERSION         Show system version\n");
    crate::print!("  ABOUT                About Rootleaf\n");
    crate::print!("  CLS, CLEAR           Clear screen\n");
    crate::print!("  ECHO <TEXT>          Print text\n");
    crate::print!("  PWD                  Print current working directory\n");
    crate::print!("  CD <PATH>            Change directory\n");
    crate::print!("  CD ..                Go to parent directory\n");
    crate::print!("  LS, DIR              List current directory\n");
    crate::print!("  LS <PATH>            List directory\n");
    crate::print!("  DIR <PATH>           List directory\n");
    crate::print!("  TYPE <FILE>          Show text file\n");
    crate::print!("  CAT <FILE>           Show text file\n");
    crate::print!("  HD <FILE>            Hexdump file\n");
    crate::print!("  HEXDUMP <FILE>       Hexdump file\n");
    crate::print!("  TOUCH <FILE>         Create empty RAMFS file\n");
    crate::print!("  WRITE <FILE> <TEXT>  Write text to RAMFS file\n");
    crate::print!("  RM <FILE>            Delete RAMFS file\n");
    crate::print!("  DEL <FILE>           Delete RAMFS file\n");
    crate::print!("  EDIT <FILE>          Launch editor for RAMFS file\n");
    crate::print!("  RUN <FILE>           Execute shell script\n");
    crate::print!("  ELF <FILE>           Load and run ELF64 binary\n");
    crate::print!("  EXEC <FILE>          Alias for ELF\n");
    crate::print!("  MOUNT <SRC><DST>[FS] Mount image or /dev/sdX device\n");
    crate::print!("  UMOUNT               Unmount /disk1\n");
    crate::print!("  DISKINFO             Show disk information\n");
    crate::print!("  MOUNTS               Show mounted filesystems\n");
    crate::print!("  LSDEV                List detected devices\n");
    crate::print!("  TASKS                Show task count\n");
    crate::print!("  MEM                  Print memory information\n");
    crate::print!("  SYSINFO              Print OS information\n");
    crate::print!("  MMAP                 Show Limine memory map\n");
    crate::print!("  REBOOT               Restart machine\n");
    crate::print!("\nLinux-style paths:\n");
    crate::print!("  /                    Virtual root\n");
    crate::print!("  /ram                 RAMFS\n");
    crate::print!("  /disk1               FAT32 disk\n");
    crate::print!("Examples:\n");
    crate::print!("  LS /\n");
    crate::print!("  LS /disk1\n");
    crate::print!("  TYPE /disk1/README.TXT\n");
    crate::print!("  ELF /disk1/APP.ELF\n");
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
    crate::print!(
        "CPU:         {}\n",
        crate::arch::x86_64::cpu::get_cpu_vendor()
    );

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("RAM:         ");
    crate::kernel::write_raw(crate::lib::u64_to_str(mem.usable_mib(), &mut buf));
    crate::print!(" MiB usable\n");

    crate::kernel::write_raw("CWD:         ");
    crate::kernel::write_raw(crate::fs::cwd::get());
    crate::print!("\n");

    crate::kernel::write_raw("Disk1:       ");
    if crate::fs::fat32::is_mounted() {
        crate::kernel::write_raw("mounted at /disk1\n");
    } else {
        crate::kernel::write_raw("not mounted\n");
    }
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

fn dir_current() {
    dir_resolved(crate::fs::cwd::get());
}

fn dir_path(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => dir_resolved(path),
        None => crate::print!("Invalid path\n"),
    }
}

fn dir_resolved(path: &str) {
    serial_log_path("[cmd] LS path", path);

    match crate::fs::vfs::parse_path(path) {
        Ok(parsed) => {
            crate::kernel::write_raw("Directory of ");
            crate::kernel::write_raw(path);
            crate::print!("\n\n");

            match parsed.backend {
                VfsBackend::Root => {
                    crate::print!("ram/\n");
                    crate::print!("disk1/\n");
                    crate::print!("dev/\n");
                    crate::print!("proc/\n");
                }

                VfsBackend::Dev => {
                    let relative = crate::fs::vfs::normalize_path(parsed.path);

                    if relative.is_empty() {
                        match core::str::from_utf8(crate::fs::vfs::build_dev_list()) {
                            Ok(text) => {
                                for line in text.lines() {
                                    crate::print!("{}\n", line);
                                }
                            }
                            Err(_) => crate::print!("(no devices)\n"),
                        }
                    } else {
                        crate::print!("Not a directory\n");
                    }
                }

                VfsBackend::Fat32 => {
                    if !crate::fs::fat32::is_mounted() {
                        crate::kernel::write_raw("disk1 is not mounted\n");
                        after_vfs_op();
                        return;
                    }

                    let relative = crate::fs::vfs::normalize_path(parsed.path);

                    crate::drivers::serial::write_str("[cmd] FAT32 ls relative='");
                    crate::drivers::serial::write_str(relative);
                    crate::drivers::serial::write_str("'\n");

                    let result = if relative.is_empty() {
                        /*
                            Root of mounted FAT32 filesystem.
                            Do NOT pass "/disk1" or "1:\" here.
                            FAT32 driver should receive a path relative to its own root.
                        */
                        crate::fs::fat32::print_dir("")
                    } else {
                        crate::fs::fat32::print_dir(relative)
                    };

                    if let Err(error) = result {
                        crate::drivers::serial::write_str("[cmd] FAT32 ls failed: ");
                        crate::drivers::serial::write_str(crate::fs::vfs::error_str(error));
                        crate::drivers::serial::write_str("\n");

                        crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                        crate::print!("\n");
                    }
                }

                VfsBackend::Ext2 => {
                    if !crate::fs::ext2::is_mounted() {
                        crate::kernel::write_raw("disk is not mounted\n");
                        after_vfs_op();
                        return;
                    }

                    let relative = crate::fs::vfs::normalize_path(parsed.path);

                    let result = if relative.is_empty() {
                        crate::fs::ext2::print_dir("")
                    } else {
                        crate::print!("Not a directory\n");
                        after_vfs_op();
                        return;
                    };

                    if let Err(error) = result {
                        crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                        crate::print!("\n");
                    }
                }

                VfsBackend::Isofs => {
                    if !crate::fs::isofs::is_mounted() {
                        crate::kernel::write_raw("disk is not mounted\n");
                        after_vfs_op();
                        return;
                    }

                    let relative = crate::fs::vfs::normalize_path(parsed.path);

                    let result = if relative.is_empty() {
                        crate::fs::isofs::print_dir("")
                    } else {
                        crate::fs::isofs::print_dir(relative)
                    };

                    if let Err(error) = result {
                        crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                        crate::print!("\n");
                    }
                }

                VfsBackend::Proc => {
                    let relative = crate::fs::vfs::normalize_path(parsed.path);

                    if relative.is_empty() {
                        crate::print!("version\n");
                        crate::print!("cpuinfo\n");
                        crate::print!("cwd\n");
                        crate::print!("pid\n");
                        crate::print!("tasks\n");
                        crate::print!("mounts\n");
                        crate::print!("meminfo\n");
                    } else {
                        crate::print!("Not a directory\n");
                    }
                }
                VfsBackend::Ramfs => {
                    let relative = crate::fs::vfs::normalize_path(parsed.path);

                    if relative.is_empty() {
                        crate::fs::ramfs::print_dir();
                    } else {
                        crate::print!("Not a directory\n");
                    }
                }
            }
        }

        Err(e) => {
            crate::kernel::write_raw(crate::fs::vfs::error_str(e));
            crate::print!("\n");
        }
    }

    after_vfs_op();
}

fn type_file(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] CAT path", path);

            if is_directory_path(path) {
                crate::print!("Cannot read a directory\n");
                return;
            }

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    if let Ok(text) = core::str::from_utf8(data) {
                        crate::kernel::write_raw(text);

                        if !text.ends_with('\n') {
                            crate::print!("\n");
                        }
                    } else {
                        crate::print!("Cannot display binary file. Use HEXDUMP instead.\n");
                    }
                }

                Err(error) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                    crate::print!("\n");
                }
            }

            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn hexdump_file(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] HEXDUMP path", path);

            match crate::fs::vfs::read(path) {
                Ok(data) => hexdump(data),

                Err(error) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                    crate::print!("\n");
                }
            }

            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn hexdump(data: &[u8]) {
    let mut offset = 0usize;

    while offset < data.len() {
        print_hex_usize(offset);
        crate::kernel::write_raw(": ");

        for i in 0..16 {
            if offset + i < data.len() {
                print_hex_byte(data[offset + i]);
                crate::kernel::write_byte(b' ');
            } else {
                crate::kernel::write_raw("   ");
            }
        }

        crate::kernel::write_raw(" |");

        for i in 0..16 {
            if offset + i < data.len() {
                let b = data[offset + i];

                if b.is_ascii_graphic() || b == b' ' {
                    crate::kernel::write_byte(b);
                } else {
                    crate::kernel::write_byte(b'.');
                }
            }
        }

        crate::kernel::write_raw("|\n");

        offset += 16;

        if offset >= 256 {
            crate::kernel::write_raw("[hexdump truncated to 256 bytes]\n");
            break;
        }
    }
}

fn echo(text: &[u8]) {
    for &b in text {
        crate::kernel::write_byte(b);
    }

    crate::print!("\n");
}

fn touch_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);

    if arg.is_empty() {
        crate::print!("Usage: TOUCH <PATH>\n");
        return;
    }

    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            if is_fat32_path(path) {
                crate::print!("TOUCH on /disk1 is disabled for now\n");
                return;
            }

            match crate::fs::vfs::write(path, b"") {
                Ok(()) => crate::print!("Created\n"),
                Err(e) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn write_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);

    let Some((path_arg, text)) = split_first_word(arg) else {
        crate::print!("Usage: WRITE <PATH> <TEXT>\n");
        return;
    };

    let mut path_buf = [0u8; 128];

    match make_absolute_path(path_arg, &mut path_buf) {
        Some(path) => {
            if is_fat32_path(path) {
                crate::print!("WRITE on /disk1 is disabled for now\n");
                return;
            }

            match crate::fs::vfs::write(path, text) {
                Ok(()) => crate::print!("Written\n"),
                Err(e) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn edit_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] EDIT path", path);

            if is_fat32_path(path) {
                crate::print!("EDIT on /disk1 is disabled for now\n");
                crate::print!("Reason: FAT32/VFS write path is not stable yet\n");
                return;
            }

            crate::shell::editor::launch(path);
        }

        None => crate::print!("Invalid path\n"),
    }
}

pub fn run_script(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] RUN path", path);

            if is_directory_path(path) {
                crate::print!("Cannot RUN a directory\n");
                return;
            }

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    if let Ok(text) = core::str::from_utf8(data) {
                        for line in text.lines() {
                            let bytes = trim_ascii(line.as_bytes());

                            if bytes.is_empty() {
                                continue;
                            }

                            execute_command(bytes);
                            after_vfs_op();
                        }
                    } else {
                        crate::print!("Script is not text\n");
                    }
                }

                Err(e) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn elf_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);

    crate::drivers::serial::write_str("ELF: raw arg = '");
    serial_write_bytes(arg);
    crate::drivers::serial::write_str("'\n");

    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            crate::drivers::serial::write_str("ELF: resolved path = '");
            crate::drivers::serial::write_str(path);
            crate::drivers::serial::write_str("'\n");

            if is_directory_path(path) {
                crate::print!("ELF path points to a directory\n");
                return;
            }

            crate::drivers::serial::write_str("ELF: before vfs::read path='");
            crate::drivers::serial::write_str(path);
            crate::drivers::serial::write_str("'\n");

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    crate::drivers::serial::write_str("ELF: vfs::read ok, size=");
                    crate::drivers::serial::write_hex(data.len());
                    crate::drivers::serial::write_str("\n");

                    match crate::kernel::elf64::run(data) {
                        Ok(code) => {
                            crate::kernel::write_raw("ELF exited with code ");

                            let mut buf = [0u8; 20];
                            crate::kernel::write_raw(crate::lib::u64_to_str(code as u64, &mut buf));

                            crate::print!("\n");
                        }

                        Err(msg) => {
                            crate::kernel::write_raw("ELF error: ");
                            crate::kernel::write_raw(msg);
                            crate::print!("\n");
                        }
                    }
                }

                Err(e) => {
                    crate::drivers::serial::write_str("ELF: vfs::read failed: ");
                    crate::drivers::serial::write_str(crate::fs::vfs::error_str(e));
                    crate::drivers::serial::write_str("\n");

                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            crate::drivers::serial::write_str("ELF: after vfs::read block\n");
            after_vfs_op();
        }

        None => {
            crate::print!("Invalid path\n");
        }
    }
}

pub fn rm_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] RM path", path);

            if is_fat32_path(path) {
                crate::print!("RM on /disk1 is disabled for now\n");
                return;
            }

            match crate::fs::vfs::delete(path) {
                Ok(()) => crate::print!("Deleted\n"),

                Err(e) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn mount_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);

    if arg.is_empty() {
        crate::print!("Usage: MOUNT <IMAGE_PATH> <TARGET> [FSTYPE]\n");
        crate::print!("   or: MOUNT /dev/sda /disk1\n");
        crate::print!("   or: MOUNT /dev/sda1 /disk1 fat32\n");
        crate::print!("   or: MOUNT /dev/sda1 /disk1 ext2\n");
        return;
    }

    // Parse up to three whitespace-separated tokens: source, target, fstype
    let mut i = 0usize;
    // source
    let mut src_end = 0usize;
    while src_end < arg.len() && !arg[src_end].is_ascii_whitespace() {
        src_end += 1;
    }
    let source = trim_ascii(&arg[..src_end]);

    // skip whitespace
    i = src_end;
    while i < arg.len() && arg[i].is_ascii_whitespace() {
        i += 1;
    }

    // target
    let mut tgt_end = i;
    while tgt_end < arg.len() && !arg[tgt_end].is_ascii_whitespace() {
        tgt_end += 1;
    }
    let target = if i < arg.len() {
        trim_ascii(&arg[i..tgt_end])
    } else {
        b""
    };

    // skip whitespace to fstype
    i = tgt_end;
    while i < arg.len() && arg[i].is_ascii_whitespace() {
        i += 1;
    }

    let fstype = if i < arg.len() {
        trim_ascii(&arg[i..])
    } else {
        b""
    };

    if !target.is_empty() {
        mount_with_target(
            source,
            target,
            if fstype.is_empty() {
                None
            } else {
                Some(fstype)
            },
        );
        return;
    }

    mount_image_from_path(source, None);
}

fn mount_ata_autodetect(disk_idx: usize, target_path: &str) {
    const PROBE_BYTES: usize = 4 * 1024;
    static mut PROBE_BUF: [u8; PROBE_BYTES] = [0u8; PROBE_BYTES];

    let mut ok = true;

    unsafe {
        let probe_ptr = core::ptr::addr_of_mut!(PROBE_BUF) as *mut u8;

        for i in 0..(PROBE_BYTES / 512) {
            let mut sec = [0u8; 512];

            if crate::drivers::ata::read_sector(disk_idx, i as u32, &mut sec).is_err() {
                ok = false;
                break;
            }

            core::ptr::copy_nonoverlapping(sec.as_ptr(), probe_ptr.add(i * 512), 512);
        }
    }

    if !ok {
        crate::print!("Failed to read disk\n");
        return;
    }

    let probe: &[u8] = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(PROBE_BUF) as *const u8, PROBE_BYTES)
    };

    let mut start_lba: u32 = 0;

    if probe[510] == 0x55 && probe[511] == 0xAA {
        let p_off = 446;

        let p_start = (probe[p_off + 8] as u32)
            | ((probe[p_off + 9] as u32) << 8)
            | ((probe[p_off + 10] as u32) << 16)
            | ((probe[p_off + 11] as u32) << 24);

        let p_len = (probe[p_off + 12] as u32)
            | ((probe[p_off + 13] as u32) << 8)
            | ((probe[p_off + 14] as u32) << 16)
            | ((probe[p_off + 15] as u32) << 24);

        if p_start != 0 && p_len != 0 {
            start_lba = p_start;
        }
    }

    // Check ext2 superblock magic at partition_start*512 + 1024 + 56
    // Superblock sits at byte 1024 from partition start.
    // We need to read it from the disk directly.
    let sb_lba = start_lba + 2; // 1024 bytes in = 2 sectors from partition start
    let mut sb_sec = [0u8; 512];
    let mut sb_sec2 = [0u8; 512];
    let is_ext2 = if crate::drivers::ata::read_sector(disk_idx, sb_lba, &mut sb_sec).is_ok()
        && crate::drivers::ata::read_sector(disk_idx, sb_lba + 1, &mut sb_sec2).is_ok()
    {
        // ext2 magic is at offset 56 within the superblock (= byte 1024+56 from partition start)
        // sector 2 of partition = bytes 0..511, magic at byte 56
        let magic = (sb_sec[56] as u16) | ((sb_sec[57] as u16) << 8);
        magic == 0xEF53
    } else {
        false
    };

    if is_ext2 {
        // Load up to 64 MiB of the partition into the ext2 buffer
        const MAX_EXT2: usize = 64 * 1024 * 1024;
        static mut EXT2_BUF: [u8; MAX_EXT2] = [0u8; MAX_EXT2];

        let total_sectors = match crate::drivers::ata::disk_sectors(disk_idx) {
            Some(s) => s as usize,
            None => {
                crate::print!("No disk\n");
                return;
            }
        };
        let part_sectors = if start_lba > 0 {
            total_sectors.saturating_sub(start_lba as usize)
        } else {
            total_sectors
        };
        let sectors_to_read = core::cmp::min(part_sectors, MAX_EXT2 / 512);

        unsafe {
            for i in 0..sectors_to_read {
                let lba = start_lba + i as u32;
                let dst = &mut EXT2_BUF[i * 512..(i + 1) * 512];
                let mut sec = [0u8; 512];
                if crate::drivers::ata::read_sector(disk_idx, lba, &mut sec).is_err() {
                    crate::print!("Read error during ext2 load\n");
                    return;
                }
                dst.copy_from_slice(&sec);
            }

            let data: &'static [u8] = &EXT2_BUF[..sectors_to_read * 512];
            if crate::fs::ext2::mount(data) {
                let _ = crate::fs::vfs::mount(target_path, crate::fs::vfs::VfsBackend::Ext2);
                if !eq_ignore_ascii_case(target_path.as_bytes(), b"disk1") {
                    let _ = crate::fs::vfs::mount("disk1", crate::fs::vfs::VfsBackend::Ext2);
                }
                crate::kernel::write_raw("Mounted ext2 disk at ");
                crate::kernel::write_raw(target_path);
                crate::print!("\n");
            } else {
                crate::print!("Failed to mount as ext2\n");
            }
        }
    } else {
        // Try FAT32
        match crate::fs::fat32::mount_ata(disk_idx) {
            Ok(()) => {
                let _ = crate::fs::vfs::mount(target_path, crate::fs::vfs::VfsBackend::Fat32);
                if !eq_ignore_ascii_case(target_path.as_bytes(), b"disk1") {
                    let _ = crate::fs::vfs::mount("disk1", crate::fs::vfs::VfsBackend::Fat32);
                }
                crate::kernel::write_raw("Mounted fat32 disk at ");
                crate::kernel::write_raw(target_path);
                crate::print!("\n");
            }
            Err(message) => {
                crate::kernel::write_raw(message);
                crate::print!("\n");
            }
        }
    }
}

#[derive(Clone, Copy)]
enum BlockDeviceKind {
    Sata,
    Ata,
}

#[derive(Clone, Copy)]
struct BlockDevice {
    kind: BlockDeviceKind,
    index: usize,
}

fn resolve_block_device(dev: &str) -> Option<BlockDevice> {
    if !dev.starts_with("sd") || dev.len() < 3 {
        return None;
    }

    let letter = dev.as_bytes()[2];

    if letter < b'a' || letter > b'z' {
        return None;
    }

    let index = (letter - b'a') as usize;

    let sata_count = crate::drivers::sata::drive_count();

    if index < sata_count {
        return Some(BlockDevice {
            kind: BlockDeviceKind::Sata,
            index,
        });
    }

    let ata = crate::drivers::pci::scan_legacy_ata();
    let ata_index = index.saturating_sub(sata_count);

    if ata_index < ata.ata_devices as usize {
        return Some(BlockDevice {
            kind: BlockDeviceKind::Ata,
            index: ata_index,
        });
    }

    None
}

fn block_read_sector(
    dev: BlockDevice,
    lba: u32,
    out: &mut [u8; 512],
) -> Result<(), &'static str> {
    match dev.kind {
        BlockDeviceKind::Sata => {
            match crate::drivers::sata::read_sector(lba as u64, out) {
                Ok(()) => Ok(()),

                Err(crate::drivers::sata::SataError::MmioNotMapped) => {
                    Err("SATA/AHCI disk access is not implemented yet")
                }

                Err(crate::drivers::sata::SataError::NoController) => {
                    Err("No SATA controller detected")
                }

                Err(crate::drivers::sata::SataError::NoDrive) => {
                    Err("No SATA drive detected")
                }

                Err(_) => Err("Failed to read SATA sector"),
            }
        }

        BlockDeviceKind::Ata => {
            crate::drivers::ata::read_sector(dev.index, lba, out)
                .map_err(|_| "Failed to read ATA sector")
        }
    }
}

fn mount_with_target(source: &[u8], target: &[u8], fstype: Option<&[u8]>) {
    let mut path_buf = [0u8; 128];

    let target_path = match make_absolute_path(target, &mut path_buf) {
        Some(path) => path,

        None => {
            crate::print!("Invalid target path\n");
            after_vfs_op();
            return;
        }
    };

    if source.starts_with(b"/dev/") {
        let Ok(devstr) = core::str::from_utf8(source) else {
            crate::print!("Invalid device path\n");
            after_vfs_op();
            return;
        };

        let dev = &devstr[5..];

        if !dev.starts_with("sd") {
            crate::print!("Unsupported device\n");
            after_vfs_op();
            return;
        }

        let Some(block_dev) = resolve_block_device(dev) else {
            crate::print!("No such block device\n");
            after_vfs_op();
            return;
        };

        /*
            Parse partition number:
                sda  -> None
                sda1 -> Some(1)
                sdb2 -> Some(2)
        */
        let mut part_idx: Option<usize> = None;

        for (i, c) in dev.chars().enumerate() {
            if c.is_ascii_digit() {
                part_idx = Some(i);
                break;
            }
        }

        /*
            Whole-disk autodetect:
                mount /dev/sda /mnt

            Only supported for legacy ATA right now. SATA read_sector() is still a stub
            until AHCI MMIO + DMA are implemented.
        */
        if part_idx.is_none() && fstype.is_none() {
            match block_dev.kind {
                BlockDeviceKind::Ata => {
                    mount_ata_autodetect(block_dev.index, target_path);
                }

                BlockDeviceKind::Sata => {
                    crate::print!("SATA/AHCI full-disk mount is not implemented yet\n");
                }
            }

            after_vfs_op();
            return;
        }

        /*
            If user provides filesystem type, allow:
                mount /dev/sda /mnt ext2
            to mean:
                mount /dev/sda1 /mnt ext2
        */
        let part = if let Some(idx) = part_idx {
            let digs = &dev[idx..];

            match digs.parse::<usize>() {
                Ok(n) if n > 0 => n,

                _ => {
                    crate::print!("Invalid device partition\n");
                    after_vfs_op();
                    return;
                }
            }
        } else if fstype.is_some() {
            1
        } else {
            crate::print!("Specify partition like /dev/sda1 or provide filesystem type\n");
            after_vfs_op();
            return;
        };

        let mut sector = [0u8; 512];

        if let Err(error) = block_read_sector(block_dev, 0, &mut sector) {
            crate::kernel::write_raw(error);
            crate::print!("\n");
            after_vfs_op();
            return;
        }

        let p_off = 446 + (part - 1) * 16;

        if p_off + 16 > 512 {
            crate::print!("Invalid partition index\n");
            after_vfs_op();
            return;
        }

        if sector[510] != 0x55 || sector[511] != 0xAA {
            crate::print!("Invalid or unsupported MBR\n");
            after_vfs_op();
            return;
        }

        let start_lba = (sector[p_off + 8] as u32)
            | ((sector[p_off + 9] as u32) << 8)
            | ((sector[p_off + 10] as u32) << 16)
            | ((sector[p_off + 11] as u32) << 24);

        let num_sectors = (sector[p_off + 12] as u32)
            | ((sector[p_off + 13] as u32) << 8)
            | ((sector[p_off + 14] as u32) << 16)
            | ((sector[p_off + 15] as u32) << 24);

        if start_lba == 0 || num_sectors == 0 {
            crate::print!("Partition not present or empty\n");
            after_vfs_op();
            return;
        }

        const MAX_BYTES: usize = 4 * 1024 * 1024;
        static mut TMP_BUF: [u8; MAX_BYTES] = [0u8; MAX_BYTES];

        let mut bytes_to_read = (num_sectors as usize).saturating_mul(512);

        if bytes_to_read > MAX_BYTES {
            bytes_to_read = MAX_BYTES;
        }

        let sectors_to_read = (bytes_to_read + 511) / 512;
        let mut written = 0usize;

        for i in 0..sectors_to_read {
            let lba = start_lba.wrapping_add(i as u32);
            let mut sec = [0u8; 512];

            if let Err(error) = block_read_sector(block_dev, lba, &mut sec) {
                crate::kernel::write_raw(error);
                crate::print!("\n");
                after_vfs_op();
                return;
            }

            unsafe {
                let copy_n = core::cmp::min(512, MAX_BYTES - written);

                let tmp_ptr = core::ptr::addr_of_mut!(TMP_BUF) as *mut u8;

                core::ptr::copy_nonoverlapping(
                    sec.as_ptr(),
                    tmp_ptr.add(written),
                    copy_n,
                );

                written += copy_n;
            }
        }

        let data = unsafe {
            core::slice::from_raw_parts(
                core::ptr::addr_of!(TMP_BUF) as *const u8,
                written,
            )
        };

        match fstype {
            Some(ft) => match core::str::from_utf8(ft) {
                Ok("ext2") | Ok("ext3") | Ok("Ext2") | Ok("Ext3") => {
                    if crate::fs::ext2::mount(data) {
                        let _ =
                            crate::fs::vfs::mount(target_path, crate::fs::vfs::VfsBackend::Ext2);

                        crate::kernel::write_raw("Mounted ext2/3 partition at ");
                        crate::kernel::write_raw(target_path);
                        crate::print!("\n");
                    } else {
                        crate::print!("Failed to mount ext2/3 partition\n");
                    }
                }

                Ok("fat32") | Ok("vfat") | Ok("FAT32") => {
                    if crate::fs::fat32::mount(data) {
                        let _ =
                            crate::fs::vfs::mount(target_path, crate::fs::vfs::VfsBackend::Fat32);

                        crate::kernel::write_raw("Mounted FAT32 partition at ");
                        crate::kernel::write_raw(target_path);
                        crate::print!("\n");
                    } else {
                        crate::print!("Failed to mount FAT32 partition\n");
                    }
                }

                Ok("isofs") | Ok("iso9660") | Ok("iso") | Ok("cdrom") => {
                    if crate::fs::isofs::mount(data) {
                        let _ =
                            crate::fs::vfs::mount(target_path, crate::fs::vfs::VfsBackend::Isofs);

                        crate::kernel::write_raw("Mounted ISO9660 partition at ");
                        crate::kernel::write_raw(target_path);
                        crate::print!("\n");
                    } else {
                        crate::print!("Failed to mount ISO9660 partition\n");
                    }
                }

                _ => {
                    crate::print!("Unsupported filesystem type for /dev mounting\n");
                }
            },

            None => {
                crate::print!("Specify filesystem type when mounting /dev devices\n");
            }
        }

        after_vfs_op();
        return;
    }

    mount_image_from_path(source, Some((target_path, fstype)));
}

fn mount_image_from_path(path_arg: &[u8], target_and_type: Option<(&str, Option<&[u8]>)>) {
    let mut path_buf = [0u8; 128];

    match make_absolute_path(path_arg, &mut path_buf) {
        Some(path) => {
            if is_directory_path(path) {
                crate::print!("Cannot mount a directory\n");
                return;
            }

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    if let Some((target_path, fstype)) = target_and_type {
                        // Explicit target provided
                        if let Some(ft) = fstype {
                            match core::str::from_utf8(ft) {
                                Ok("fat32") => {
                                    if crate::fs::fat32::mount(data) {
                                        let _ = crate::fs::vfs::mount(
                                            target_path,
                                            crate::fs::vfs::VfsBackend::Fat32,
                                        );
                                        crate::kernel::write_raw("Mounted FAT32 image at ");
                                        crate::kernel::write_raw(target_path);
                                        crate::print!("\n");
                                    } else {
                                        crate::print!("Failed to mount image as FAT32\n");
                                    }
                                }
                                Ok("Ext2") => {
                                    if crate::fs::ext2::mount(data) {
                                        let _ = crate::fs::vfs::mount(
                                            target_path,
                                            crate::fs::vfs::VfsBackend::Ext2,
                                        );
                                        crate::kernel::write_raw("Mounted Ext2 image at ");
                                        crate::kernel::write_raw(target_path);
                                        crate::print!("\n");
                                    } else {
                                        crate::print!("Failed to mount image as Ext2\n");
                                    }
                                }
                                Ok("isofs") | Ok("iso9660") | Ok("iso") => {
                                    if crate::fs::isofs::mount(data) {
                                        let _ = crate::fs::vfs::mount(
                                            target_path,
                                            crate::fs::vfs::VfsBackend::Isofs,
                                        );
                                        crate::kernel::write_raw("Mounted ISO9660 image at ");
                                        crate::kernel::write_raw(target_path);
                                        crate::print!("\n");
                                    } else {
                                        crate::print!("Failed to mount image as ISO9660\n");
                                    }
                                }
                                Ok("cdrom") => {
                                    if crate::fs::isofs::mount(data) {
                                        let _ = crate::fs::vfs::mount(
                                            target_path,
                                            crate::fs::vfs::VfsBackend::Isofs,
                                        );
                                        crate::kernel::write_raw("Mounted CD-ROM image at ");
                                        crate::kernel::write_raw(target_path);
                                        crate::print!("\n");
                                    } else {
                                        crate::print!("Failed to mount image as CD-ROM\n");
                                    }
                                }
                                Ok("ramfs") | Ok("ram") => {
                                    let _ = crate::fs::vfs::mount(
                                        target_path,
                                        crate::fs::vfs::VfsBackend::Ramfs,
                                    );
                                    crate::kernel::write_raw("Mounted ramfs at ");
                                    crate::kernel::write_raw(target_path);
                                    crate::print!("\n");
                                }
                                Ok("dev") => {
                                    let _ = crate::fs::vfs::mount(
                                        target_path,
                                        crate::fs::vfs::VfsBackend::Dev,
                                    );
                                    crate::kernel::write_raw("Mounted devfs at ");
                                    crate::kernel::write_raw(target_path);
                                    crate::print!("\n");
                                }
                                Ok("proc") => {
                                    let _ = crate::fs::vfs::mount(
                                        target_path,
                                        crate::fs::vfs::VfsBackend::Proc,
                                    );
                                    crate::kernel::write_raw("Mounted procfs at ");
                                    crate::kernel::write_raw(target_path);
                                    crate::print!("\n");
                                }
                                _ => {
                                    crate::print!("Unsupported filesystem type\n");
                                }
                            }
                        } else {
                            // No fstype provided; attempt to auto-detect
                            if is_probably_fat32(data) {
                                if crate::fs::fat32::mount(data) {
                                    let _ = crate::fs::vfs::mount(
                                        target_path,
                                        crate::fs::vfs::VfsBackend::Fat32,
                                    );
                                    crate::kernel::write_raw("Auto-detected FAT32 and mounted at ");
                                    crate::kernel::write_raw(target_path);
                                    crate::print!("\n");
                                } else {
                                    crate::print!("Failed to mount image\n");
                                }
                            } else if is_probably_isofs(data) {
                                if crate::fs::isofs::mount(data) {
                                    let _ = crate::fs::vfs::mount(
                                        target_path,
                                        crate::fs::vfs::VfsBackend::Isofs,
                                    );
                                    crate::kernel::write_raw(
                                        "Auto-detected CD-ROM/ISO9660 and mounted at ",
                                    );
                                    crate::kernel::write_raw(target_path);
                                    crate::print!("\n");
                                } else {
                                    crate::print!("Failed to mount image\n");
                                }
                            } else {
                                crate::print!("Unknown or unsupported filesystem for image\n");
                            }
                        }
                    } else {
                        // legacy behavior: mount image as /disk1
                        if crate::fs::fat32::mount(data) {
                            crate::print!("Mounted FAT32 image at /disk1\n");
                        } else {
                            crate::print!("Failed to mount image\n");
                        }
                    }
                }

                Err(error) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                    crate::print!("\n");
                }
            }

            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn is_probably_fat32(data: &[u8]) -> bool {
    if data.len() < 512 {
        return false;
    }
    if data[510] != 0x55 || data[511] != 0xAA {
        return false;
    }
    let bps = (data[11] as u16) | ((data[12] as u16) << 8);
    let spc = data[13];
    let reserved = (data[14] as u16) | ((data[15] as u16) << 8);
    let fats = data[16];
    bps != 0 && spc != 0 && reserved != 0 && fats != 0
}

fn is_probably_isofs(data: &[u8]) -> bool {
    let sector = 16 * 2048;

    if data.len() < sector + 6 {
        return false;
    }

    data[sector] == 1 && &data[sector + 1..sector + 6] == b"CD001"
}

fn umount_cmd() {
    crate::fs::fat32::unmount();
    crate::print!("Unmounted /disk1\n");
}

fn diskinfo() {
    crate::print!("Disks\n");
    crate::print!("=====\n\n");

    crate::kernel::write_raw("ram:    RAMFS mounted at /ram\n");

    let pci = crate::drivers::pci::scan_storage();
    let ata = crate::drivers::pci::scan_legacy_ata();

    let has_storage =
        pci.total() > 0 || ata.channels > 0 || ata.ata_devices > 0 || ata.atapi_devices > 0;

    if crate::fs::fat32::is_mounted() {
        crate::kernel::write_raw("disk1:  FAT32 mounted at /disk1\n");
    } else if has_storage {
        crate::kernel::write_raw("disk1:  present, not mounted\n");
    } else {
        crate::kernel::write_raw("disk1:  not present\n");
    }
}

fn mounts() {
    crate::print!("Mounted filesystems:\n");
    match crate::fs::vfs::read("/etc/mtab") {
        Ok(data) => {
            if let Ok(text) = core::str::from_utf8(data) {
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    crate::kernel::write_raw("  ");
                    crate::kernel::write_raw(line);
                    crate::print!("\n");
                }
            } else {
                crate::print!("  (mount table is not valid UTF-8)\n");
            }
        }
        Err(_) => crate::print!("  (mount table unavailable)\n"),
    }
}

fn tasks() {
    crate::kernel::write_raw("Task count: ");

    let mut buf = [0u8; 20];
    crate::kernel::write_raw(crate::lib::u64_to_str(
        crate::scheduler::task_count() as u64,
        &mut buf,
    ));

    crate::print!("\n");
}

fn lsdev() {
    crate::print!("Devices\n");
    crate::print!("=======\n\n");

    crate::kernel::write_raw("CPU:             ");
    crate::kernel::write_raw(crate::arch::x86_64::cpu::get_cpu_vendor());
    crate::print!("\n");

    crate::kernel::write_raw("Interrupt Ctrl:  8259 PIC\n");
    crate::kernel::write_raw("Keyboard:        PS/2 (IRQ1)\n");
    crate::kernel::write_raw("Display:         Limine framebuffer\n");
    crate::kernel::write_raw("Console:         Rootleaf framebuffer console\n");
    crate::kernel::write_raw("Serial:          COM1 (0x3F8)\n");

    let st = crate::drivers::pci::scan_storage();
    let ata = crate::drivers::pci::scan_legacy_ata();

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("Storage Ctrl:    IDE: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(st.ide as u64, &mut buf));

    let mut buf = [0u8; 20];

    let mut buf = [0u8; 20];

    let pci = crate::drivers::pci::scan_storage();
    crate::kernel::write_raw(", SATA/AHCI controllers: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(pci.sata as u64, &mut buf));

    let sata_disks = crate::drivers::sata::drive_count();

    let mut buf = [0u8; 20];

    crate::kernel::write_raw(", SATA disks: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(sata_disks as u64, &mut buf));

    let mut buf = [0u8; 20];

    crate::kernel::write_raw(", SCSI: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(st.scsi as u64, &mut buf));

    let mut buf = [0u8; 20];

    crate::kernel::write_raw(", NVMe: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(st.nvme as u64, &mut buf));

    let mut buf = [0u8; 20];

    crate::kernel::write_raw(", Other: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(st.other as u64, &mut buf));
    crate::print!("\n");

    let mut buf = [0u8; 20];

    crate::kernel::write_raw("Legacy ATA:      Channels: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(ata.channels as u64, &mut buf));

    let mut buf = [0u8; 20];

    crate::kernel::write_raw(", ATA drives: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(ata.ata_devices as u64, &mut buf));

    let mut buf = [0u8; 20];

    crate::kernel::write_raw(", ATAPI drives: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(ata.atapi_devices as u64, &mut buf));
    crate::print!("\n");

    if ata.atapi_devices > 0 {
        crate::print!("CD-ROM:          ATAPI device detected\n");
    }
}

fn cd_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);

    if arg.is_empty() {
        let _ = crate::fs::cwd::set("/");
        return;
    }

    if arg == b"." {
        return;
    }

    if arg == b".." {
        cd_parent();
        return;
    }

    let mut buf = [0u8; 128];

    match make_absolute_path(arg, &mut buf) {
        Some(path) => match crate::fs::vfs::parse_path(path) {
            Ok(_) => {
                let normalized = ensure_no_trailing_slash_except_root(path);
                let _ = crate::fs::cwd::set(normalized);
            }

            Err(e) => {
                crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                crate::print!("\n");
            }
        },

        None => crate::print!("Invalid path\n"),
    }
}

fn cd_parent() {
    let cwd = crate::fs::cwd::get();

    if cwd == "/" {
        return;
    }

    let bytes = cwd.as_bytes();
    let mut end = bytes.len();

    while end > 1 && bytes[end - 1] == b'/' {
        end -= 1;
    }

    let mut slash = 0usize;

    for i in 0..end {
        if bytes[i] == b'/' {
            slash = i;
        }
    }

    let new_path = if slash == 0 {
        "/"
    } else {
        match core::str::from_utf8(&bytes[..slash]) {
            Ok(s) => s,
            Err(_) => "/",
        }
    };

    let _ = crate::fs::cwd::set(new_path);
}

fn ensure_no_trailing_slash_except_root(path: &str) -> &str {
    if path == "/" {
        return path;
    }

    let mut end = path.len();

    while end > 1 && path.as_bytes()[end - 1] == b'/' {
        end -= 1;
    }

    &path[..end]
}

fn is_fat32_path(path: &str) -> bool {
    path == "/disk1" || path.starts_with("/disk1/") || path.starts_with("1:")
}

fn is_directory_path(path: &str) -> bool {
    path == "/" || path.ends_with('/')
}

fn split_first_word(input: &[u8]) -> Option<(&[u8], &[u8])> {
    let input = trim_ascii(input);

    if input.is_empty() {
        return None;
    }

    let mut split = 0usize;

    while split < input.len() && !input[split].is_ascii_whitespace() {
        split += 1;
    }

    let first = &input[..split];
    let rest = if split < input.len() {
        trim_ascii(&input[split..])
    } else {
        b""
    };

    Some((first, rest))
}

fn print_hex_byte(byte: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    crate::kernel::write_byte(HEX[(byte >> 4) as usize]);
    crate::kernel::write_byte(HEX[(byte & 0x0f) as usize]);
}

fn print_hex_usize(value: usize) {
    crate::kernel::write_raw("0x");

    let mut started = false;

    for i in (0..core::mem::size_of::<usize>() * 2).rev() {
        let nibble = ((value >> (i * 4)) & 0x0f) as u8;

        if nibble != 0 || started || i == 0 {
            started = true;
            print_hex_nibble(nibble);
        }
    }
}

fn print_hex_nibble(nibble: u8) {
    let ch = match nibble {
        0..=9 => b'0' + nibble,
        10..=15 => b'a' + (nibble - 10),
        _ => b'?',
    };

    crate::kernel::write_byte(ch);
}

fn serial_log_path(label: &str, path: &str) {
    crate::drivers::serial::write_str(label);
    crate::drivers::serial::write_str(" = '");
    crate::drivers::serial::write_str(path);
    crate::drivers::serial::write_str("'\n");
}

fn serial_write_bytes(bytes: &[u8]) {
    match core::str::from_utf8(bytes) {
        Ok(s) => crate::drivers::serial::write_str(s),

        Err(_) => {
            for &b in bytes {
                if b.is_ascii_graphic()
                    || b == b' '
                    || b == b'\\'
                    || b == b'/'
                    || b == b':'
                    || b == b'.'
                    || b == b'_'
                    || b == b'-'
                {
                    crate::drivers::serial::write_byte(b);
                } else {
                    crate::drivers::serial::write_byte(b'?');
                }
            }
        }
    }
}

fn after_vfs_op() {
    crate::kernel::present();

    crate::scheduler::yield_now();

    unsafe {
        core::arch::asm!("pause", options(nomem, nostack));
    }
}
