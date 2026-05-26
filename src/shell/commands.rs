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

    if starts_with_ignore_ascii_case(command, b"MOUNT ") {
        mount_cmd(&command[6..]);
        return;
    }

    if eq_ignore_ascii_case(command, b"UMOUNT") {
        umount_cmd();
        return;
    }

    if eq_ignore_ascii_case(command, b"DISKINFO") {
        diskinfo();
        return;
    }

    if eq_ignore_ascii_case(command, b"LSDEV") {
        lsdev();
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

    if starts_with_ignore_ascii_case(command, b"RMD ") {
        rmd_cmd(&command[4..]);
        return;
    }

    crate::print!("Bad command or file name\n");
}

fn help() {
    crate::print!("Available commands:\n");
    crate::print!("  HELP              Show this help\n");
    crate::print!("  VER               Show system version\n");
    crate::print!("  CLS               Clear screen\n");
    crate::print!("  CLEAR             Clear screen\n");
    crate::print!("  ECHO <TEXT>       Print text\n");
    crate::print!("  ABOUT             About Rootleaf\n");
    crate::print!("  PWD               Print current working directory\n");
    crate::print!("  MEM               Print memory info\n");
    crate::print!("  SYSINFO           Print OS info\n");
    crate::print!("  MMAP              Show Limine memory map\n");
    crate::print!("  LSDEV             List detected devices\n");
    crate::print!("  DIR               List current directory\n");
    crate::print!("  DIR <PATH>        List directory\n");
    crate::print!("  LS                List current directory\n");
    crate::print!("  LS <PATH>         List directory\n");
    crate::print!("  TYPE <FILE>       Show the contents of a file\n");
    crate::print!("  MOUNT <SRC> [DST] Mount FAT32 image or physical disk\n");
    crate::print!("  UMOUNT            Unmount disk\n");
    crate::print!("  REBOOT            Restart machine\n");
    crate::print!("  EDIT <PATH>       Launch in-kernel editor for RAMFS files\n");
    crate::print!("  RUN <PATH>        Execute file as script\n");
    crate::print!("  ELF <PATH>        Load and run ELF64 binary\n");
    crate::print!("  RMD <PATH>        Delete RAMFS file\n");
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
    serial_log_path("[cmd] DIR path", path);

    match crate::fs::vfs::parse_path(path) {
        Ok(parsed) => {
            crate::kernel::write_raw(" Directory of ");
            crate::kernel::write_raw(path);
            crate::print!("\n\n");

            match parsed.disk {
                0 => crate::fs::ramfs::print_dir(),

                1 => {
                    crate::drivers::serial::write_str("[cmd] DIR FAT32 begin\n");
                    let result = crate::fs::fat32::print_dir(parsed.path);
                    crate::drivers::serial::write_str("[cmd] DIR FAT32 end\n");

                    if let Err(error) = result {
                        crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                        crate::print!("\n");
                    }
                }

                _ => {
                    crate::kernel::write_raw("Unsupported disk\n");
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
            serial_log_path("[cmd] TYPE path", path);

            if is_directory_path(path) {
                crate::print!("Cannot TYPE a directory\n");
                return;
            }

            crate::drivers::serial::write_str("[cmd] TYPE vfs::read begin\n");

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    crate::drivers::serial::write_str("[cmd] TYPE vfs::read ok, size ");
                    crate::drivers::serial::write_hex(data.len());
                    crate::drivers::serial::write_str("\n");

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
                    crate::drivers::serial::write_str("[cmd] TYPE vfs::read error: ");
                    crate::drivers::serial::write_str(crate::fs::vfs::error_str(error));
                    crate::drivers::serial::write_str("\n");

                    crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                    crate::print!("\n");
                }
            }

            crate::drivers::serial::write_str("[cmd] TYPE vfs::read end\n");
            after_vfs_op();
        }

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

fn edit_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] EDIT path", path);

            if is_disk1_path(path) {
                crate::print!("EDIT on disk 1 is disabled for now\n");
                crate::print!("Reason: FAT32/VFS write path is not stable yet\n");
                crate::drivers::serial::write_str("[cmd] EDIT blocked on disk 1\n");
                return;
            }

            crate::shell::editor::launch(path);
        }

        None => {
            crate::print!("Invalid path\n");
        }
    }
}

fn run_script(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] RUN path", path);

            if is_directory_path(path) {
                crate::print!("Cannot RUN a directory\n");
                return;
            }

            crate::drivers::serial::write_str("[cmd] RUN vfs::read begin\n");

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    crate::drivers::serial::write_str("[cmd] RUN vfs::read ok, size ");
                    crate::drivers::serial::write_hex(data.len());
                    crate::drivers::serial::write_str("\n");

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
                    crate::drivers::serial::write_str("[cmd] RUN vfs::read error: ");
                    crate::drivers::serial::write_str(crate::fs::vfs::error_str(e));
                    crate::drivers::serial::write_str("\n");

                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            crate::drivers::serial::write_str("[cmd] RUN vfs::read end\n");
            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn elf_cmd(arg: &[u8]) {
    crate::drivers::serial::write_str("ELF: command received\n");

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

            crate::drivers::serial::write_str("ELF: vfs::read begin\n");

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    crate::drivers::serial::write_str("ELF: file read ok, size ");
                    crate::drivers::serial::write_hex(data.len());
                    crate::drivers::serial::write_str("\n");

                    match crate::kernel::elf64::run(data) {
                        Ok(code) => {
                            crate::drivers::serial::write_str("ELF: run returned ok\n");

                            crate::kernel::write_raw("ELF exited with code ");
                            let mut buf = [0u8; 20];
                            crate::kernel::write_raw(crate::lib::u64_to_str(code as u64, &mut buf));
                            crate::print!("\n");
                        }

                        Err(msg) => {
                            crate::drivers::serial::write_str("ELF: run returned error: ");
                            crate::drivers::serial::write_str(msg);
                            crate::drivers::serial::write_str("\n");

                            crate::kernel::write_raw("ELF error: ");
                            crate::kernel::write_raw(msg);
                            crate::print!("\n");
                        }
                    }
                }

                Err(e) => {
                    crate::drivers::serial::write_str("ELF: vfs read failed for path '");
                    crate::drivers::serial::write_str(path);
                    crate::drivers::serial::write_str("': ");
                    crate::drivers::serial::write_str(crate::fs::vfs::error_str(e));
                    crate::drivers::serial::write_str("\n");

                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            crate::drivers::serial::write_str("ELF: vfs::read end\n");
            after_vfs_op();
        }

        None => {
            crate::drivers::serial::write_str("ELF: invalid path\n");
            crate::print!("Invalid path\n");
        }
    }
}

fn rmd_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);
    let mut path_buf = [0u8; 128];

    match make_absolute_path(arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] RMD path", path);

            if is_disk1_path(path) {
                crate::print!("RMD on disk 1 is disabled for now\n");
                crate::print!("Reason: FAT32/VFS write path is not stable yet\n");
                crate::drivers::serial::write_str("[cmd] RMD blocked on disk 1\n");
                return;
            }

            crate::drivers::serial::write_str("[cmd] RMD vfs::delete begin\n");

            match crate::fs::vfs::delete(path) {
                Ok(()) => crate::print!("Deleted\n"),

                Err(e) => {
                    crate::drivers::serial::write_str("[cmd] RMD vfs::delete error: ");
                    crate::drivers::serial::write_str(crate::fs::vfs::error_str(e));
                    crate::drivers::serial::write_str("\n");

                    crate::kernel::write_raw(crate::fs::vfs::error_str(e));
                    crate::print!("\n");
                }
            }

            crate::drivers::serial::write_str("[cmd] RMD vfs::delete end\n");
            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn mount_cmd(arg: &[u8]) {
    let arg = trim_ascii(arg);

    if arg.is_empty() {
        crate::print!("Usage: MOUNT <IMAGE_PATH>\n");
        crate::print!("   or: MOUNT \\\\DISK1 1:\\\n");
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

    crate::drivers::serial::write_str("[cmd] MOUNT source='");
    serial_write_bytes(source);
    crate::drivers::serial::write_str("' target='");
    serial_write_bytes(target);
    crate::drivers::serial::write_str("'\n");

    if !target.is_empty() {
        mount_with_target(source, target);
        return;
    }

    mount_image_from_path(source);
}

fn mount_with_target(source: &[u8], target: &[u8]) {
    if !eq_ignore_ascii_case(target, b"1:\\") && !eq_ignore_ascii_case(target, b"1:") {
        crate::print!("Only target 1:\\ is supported\n");
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

        crate::drivers::serial::write_str("[cmd] MOUNT physical ATA begin\n");

        match crate::fs::fat32::mount_first_ata() {
            Ok(()) => {
                crate::print!("Mounted physical ATA disk as 1:\\\n");
            }

            Err(message) => {
                crate::kernel::write_raw(message);
                crate::print!("\n");
            }
        }

        crate::drivers::serial::write_str("[cmd] MOUNT physical ATA end\n");
        after_vfs_op();
        return;
    }

    mount_image_from_path(source);
}

fn mount_image_from_path(path_arg: &[u8]) {
    let mut path_buf = [0u8; 128];

    match make_absolute_path(path_arg, &mut path_buf) {
        Some(path) => {
            serial_log_path("[cmd] MOUNT image path", path);

            if is_directory_path(path) {
                crate::print!("Cannot mount a directory\n");
                return;
            }

            crate::drivers::serial::write_str("[cmd] MOUNT image vfs::read begin\n");

            match crate::fs::vfs::read(path) {
                Ok(data) => {
                    crate::drivers::serial::write_str("[cmd] MOUNT image read ok, size ");
                    crate::drivers::serial::write_hex(data.len());
                    crate::drivers::serial::write_str("\n");

                    if crate::fs::fat32::mount(data) {
                        crate::print!("Mounted FAT32 image on disk 1\n");
                    } else {
                        crate::print!("Failed to mount image\n");
                    }
                }

                Err(error) => {
                    crate::drivers::serial::write_str("[cmd] MOUNT image read error: ");
                    crate::drivers::serial::write_str(crate::fs::vfs::error_str(error));
                    crate::drivers::serial::write_str("\n");

                    crate::kernel::write_raw(crate::fs::vfs::error_str(error));
                    crate::print!("\n");
                }
            }

            crate::drivers::serial::write_str("[cmd] MOUNT image vfs::read end\n");
            after_vfs_op();
        }

        None => crate::print!("Invalid path\n"),
    }
}

fn umount_cmd() {
    crate::fs::fat32::unmount();
    crate::print!("Unmounted disk 1\n");
}

fn diskinfo() {
    crate::print!("Disks\n");
    crate::print!("====\n\n");

    crate::kernel::write_raw("Disk 0: RAMFS\n");

    let pci = crate::drivers::pci::scan_storage();
    let ata = crate::drivers::pci::scan_legacy_ata();

    let has_storage =
        pci.total() > 0 || ata.channels > 0 || ata.ata_devices > 0 || ata.atapi_devices > 0;

    if crate::fs::fat32::is_mounted() {
        crate::kernel::write_raw("Disk 1: FAT32 (mounted)\n");
    } else if has_storage {
        crate::kernel::write_raw("Disk 1: Present (controller detected, not mounted)\n");
    } else {
        crate::kernel::write_raw("Disk 1: Not present\n");
    }
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
    crate::kernel::write_raw(", SATA/AHCI: ");

    let mut buf = [0u8; 20];
    crate::kernel::write_raw(crate::lib::u64_to_str(st.sata as u64, &mut buf));
    crate::kernel::write_raw(", SCSI: ");

    let mut buf = [0u8; 20];
    crate::kernel::write_raw(crate::lib::u64_to_str(st.scsi as u64, &mut buf));
    crate::kernel::write_raw(", NVMe: ");

    let mut buf = [0u8; 20];
    crate::kernel::write_raw(crate::lib::u64_to_str(st.nvme as u64, &mut buf));
    crate::kernel::write_raw(", Other: ");

    let mut buf = [0u8; 20];
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

    crate::kernel::write_raw("Disk 0:          RAMFS (writable)\n");

    if crate::fs::fat32::is_mounted() {
        crate::kernel::write_raw("Disk 1:          FAT32 (mounted)\n");
    } else if st.total() > 0 || ata.channels > 0 || ata.ata_devices > 0 || ata.atapi_devices > 0 {
        crate::kernel::write_raw("Disk 1:          Present (controller detected, not mounted)\n");
    } else {
        crate::kernel::write_raw("Disk 1:          Not detected\n");
    }
}

fn cd_cmd(arg: &[u8]) {
    let a = trim_ascii(arg);

    if a.is_empty() {
        let cur = crate::fs::cwd::get();

        let mut buf = [0u8; 128];
        let mut len = 0usize;

        for &b in cur.as_bytes() {
            if len >= buf.len() {
                break;
            }

            buf[len] = b;
            len += 1;
        }

        if len == 0 || buf[len - 1] != b'\\' {
            if len < buf.len() {
                buf[len] = b'\\';
                len += 1;
            }
        }

        if let Ok(s) = core::str::from_utf8(&buf[..len]) {
            let _ = crate::fs::cwd::set(s);
        }

        return;
    }

    if a == b".." {
        let cur = crate::fs::cwd::get();

        let mut buf = [0u8; 128];
        let mut len = 0usize;

        for &b in cur.as_bytes() {
            if len >= buf.len() {
                break;
            }

            buf[len] = b;
            len += 1;
        }

        if len > 0 && buf[len - 1] == b'\\' && len > 3 {
            len -= 1;
        }

        let mut pos = 0usize;

        for i in 0..len {
            if buf[i] == b'\\' {
                pos = i;
            }
        }

        let new_len = if pos + 1 < 3 { 3 } else { pos + 1 };

        if new_len <= buf.len() {
            if let Ok(s) = core::str::from_utf8(&buf[..new_len]) {
                let _ = crate::fs::cwd::set(s);
            }
        }

        return;
    }

    let mut buf = [0u8; 128];

    match make_absolute_path(a, &mut buf) {
        Some(path) => {
            serial_log_path("[cmd] CD path", path);

            let mut buf2 = [0u8; 128];
            let mut len2 = 0usize;

            for &b in path.as_bytes() {
                if len2 >= buf2.len() {
                    break;
                }

                buf2[len2] = b;
                len2 += 1;
            }

            if len2 == 0 || buf2[len2 - 1] != b'\\' {
                if len2 < buf2.len() {
                    buf2[len2] = b'\\';
                    len2 += 1;
                }
            }

            let path_owned = match core::str::from_utf8(&buf2[..len2]) {
                Ok(s) => s,

                Err(_) => {
                    crate::print!("Invalid path\n");
                    return;
                }
            };

            match crate::fs::vfs::parse_path(path_owned) {
                Ok(_) => {
                    let _ = crate::fs::cwd::set(path_owned);
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

fn is_disk1_path(path: &str) -> bool {
    let bytes = path.as_bytes();

    bytes.len() >= 2 && bytes[0] == b'1' && bytes[1] == b':'
}

fn is_directory_path(path: &str) -> bool {
    path.as_bytes().last().copied() == Some(b'\\')
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
                if b.is_ascii_graphic() || b == b' ' || b == b'\\' || b == b':' || b == b'.' {
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