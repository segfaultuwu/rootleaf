pub fn show_help_tab() {
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
    crate::print!("  DIR     List files\n");
    crate::print!("  TYPE    Show file contents\n");
    crate::print!("  CLS     Clear screen\n");
    crate::print!("  CLEAR   Clear screen\n");
    crate::print!("  ECHO    Print text\n");
    crate::print!("  PWD     Print current directory\n");
    crate::print!("  REBOOT  Restart machine\n\n");

    crate::kernel::prompt();
}

pub fn show_mem_tab() {
    crate::kernel::clear_console();

    let mem = crate::memory::info::memory_info();

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

pub fn show_disks_tab() {
    crate::kernel::clear_console();

    crate::print!("Disks\n");
    crate::print!("=====\n\n");

    crate::print!("Disk 0: RAMFS\n");
    crate::print!("Disk 1: Not present\n\n");

    crate::print!("Planned:\n");
    crate::print!("  - AHCI/SATA detection\n");
    crate::print!("  - NVMe detection\n");
    crate::print!("  - FAT32 reader\n");
    crate::print!("  - DIR command from real filesystem\n\n");

    crate::kernel::prompt();
}
