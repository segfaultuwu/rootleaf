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
        let arg = if command.len() > 2 { &command[2..] } else { b"" };
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

    crate::print!("Bad command or file name\n");
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
    crate::print!("  MOUNT <SRC> [DST]    Mount FAT32 image or physical disk\n");
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
    crate::print!("CPU:         {}\n", crate::arch::x86_64::cpu::get_cpu_vendor());

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

                VfsBackend::Ramfs => {
                    crate::fs::ramfs::print_dir();
                }

                VfsBackend::Fat32 => {
                    if !crate::fs::fat32::is_mounted() {
                        crate::kernel::write_raw("disk1 is not mounted\n");
                        after_vfs_op();
                        return;
                    }

                    let relative = crate::fs::vfs::normalize_path(parsed.path);

                    let result = if relative.is_empty() {
                        crate::fs::fat32::print_dir("")
                            .or_else(|_| crate::fs::fat32::print_dir("\\"))
                            .or_else(|_| crate::fs::fat32::print_dir("1:\\"))
                    } else {
                        crate::fs::fat32::print_dir(relative)
                    };

                    if let Err(error) = result {
                        crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                        crate::print!("\n");
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
        crate::print!("Usage: MOUNT <IMAGE_PATH>\n");
        crate::print!("   or: MOUNT DISK1 /disk1\n");
        crate::print!("   or: MOUNT \\\\DISK1 /disk1\n");
        return;
    }

    let mut split = 0usize;

    while split < arg.len() && !arg[split].is_ascii_whitespace() {
        split += 1;
    }

    let source = trim_ascii(&arg[..split]);
    let target = if split < arg.len() {
        trim_ascii(&arg[split..])
    } else {
        b""
    };

    if !target.is_empty() {
        mount_with_target(source, target);
        return;
    }

    mount_image_from_path(source);
}

fn mount_with_target(source: &[u8], target: &[u8]) {
    if !eq_ignore_ascii_case(target, b"/disk1")
        && !eq_ignore_ascii_case(target, b"1:\\")
        && !eq_ignore_ascii_case(target, b"1:")
    {
        crate::print!("Only target /disk1 is supported\n");
        return;
    }

    if eq_ignore_ascii_case(source, b"\\DISK1")
        || eq_ignore_ascii_case(source, b"\\\\DISK1")
        || eq_ignore_ascii_case(source, b"DISK1")
    {
        let st = crate::drivers::pci::scan_storage();
        let ata = crate::drivers::pci::scan_legacy_ata();
        let detected =
            st.total() > 0 || ata.channels > 0 || ata.ata_devices > 0 || ata.atapi_devices > 0;

        if !detected {
            crate::print!("Disk 1 is not detected\n");
            return;
        }

        match crate::fs::fat32::mount_first_ata() {
            Ok(()) => crate::print!("Mounted physical ATA disk at /disk1\n"),

            Err(message) => {
                crate::kernel::write_raw(message);
                crate::print!("\n");
            }
        }

        after_vfs_op();
        return;
    }

    mount_image_from_path(source);
}

fn mount_image_from_path(path_arg: &[u8]) {
    let mut path_buf = [0u8; 128];

    match make_absolute_path(path_arg, &mut path_buf) {
        Some(path) => {
            if is_directory_path(path) {
                crate::print!("Cannot mount a directory\n");
                return;
            }

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    if crate::fs::fat32::mount(data) {
                        crate::print!("Mounted FAT32 image at /disk1\n");
                    } else {
                        crate::print!("Failed to mount image\n");
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
    crate::print!("  rootfs    /\n");
    crate::print!("  ramfs     /ram\n");

    if crate::fs::fat32::is_mounted() {
        crate::print!("  fat32     /disk1\n");
    } else {
        crate::print!("  disk1     not mounted\n");
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

    crate::kernel::write_raw(", SATA/AHCI: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(st.sata as u64, &mut buf));

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
        Some(path) => {
            match crate::fs::vfs::parse_path(path) {
                Ok(_) => {
                    let normalized = ensure_no_trailing_slash_except_root(path);
                    let _ = crate::fs::cwd::set(normalized);
                }

                Err(e) => {
                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }
        }

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