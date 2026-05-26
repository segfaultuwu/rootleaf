pub fn show_help_tab() {
    crate::kernel::clear_console();

    crate::print!("Rootleaf Help\n");
    crate::print!("=============\n\n");

    crate::print!("Keyboard shortcuts:\n");
    crate::print!("  F1      Help tab\n");
    crate::print!("  F2      Memory tab\n");
    crate::print!("  F3      Disks tab\n");
    crate::print!("  ESC     Return to shell\n\n");
    crate::print!("Virtual filesystems:\n");
    crate::print!("  /dev/null      Empty device\n");
    crate::print!("  /dev/zero      Zero device\n");
    crate::print!("  /proc/version  Kernel version\n");
    crate::print!("  /proc/cpuinfo  CPU info\n\n");

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
    let pci = crate::drivers::pci::scan_storage();
    let ata = crate::drivers::pci::scan_legacy_ata();

    if crate::fs::fat32::is_mounted() {
        crate::print!("Disk 1: FAT32 (mounted)\n\n");
    } else if pci.total() > 0 || ata.channels > 0 || ata.ata_devices > 0 || ata.atapi_devices > 0 {
        crate::print!("Disk 1: Present (controller detected, not mounted)\n\n");
    } else {
        crate::print!("Disk 1: Not present\n\n");
    }

    let mut buf = [0u8; 20];
    crate::kernel::write_raw("- PCI IDE: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(pci.ide as u64, &mut buf));
    crate::kernel::write_raw(", SATA/AHCI: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(pci.sata as u64, &mut buf));
    crate::kernel::write_raw(", SCSI: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(pci.scsi as u64, &mut buf));
    crate::kernel::write_raw(", NVMe: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(pci.nvme as u64, &mut buf));
    crate::print!("\n");

    crate::kernel::write_raw("- Legacy ATA channels: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(ata.channels as u64, &mut buf));
    crate::kernel::write_raw(", ATA: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(ata.ata_devices as u64, &mut buf));
    crate::kernel::write_raw(", ATAPI: ");
    crate::kernel::write_raw(crate::lib::u64_to_str(ata.atapi_devices as u64, &mut buf));
    crate::print!("\n");

    crate::kernel::prompt();
}
