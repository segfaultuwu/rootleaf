use crate::drivers::{pci, sata};

pub fn detect() -> usize {
    pci::scan_for_sata()
}

pub fn init() {
    sata::init();
    let count = detect();

    if count > 0 {
        let msg = crate::format!("AHCI: detected {} controller(s)\n", count);
        crate::drivers::serial::write_str(msg.as_str());
    } else {
        crate::drivers::serial::write_str("AHCI: no controllers detected\n");
    }

    crate::drivers::serial::write_str("[debug] SATA drive_count=");
    crate::drivers::serial::write_hex(crate::drivers::sata::drive_count());
    crate::drivers::serial::write_str("\n");

    let ata = crate::drivers::pci::scan_legacy_ata();

    crate::drivers::serial::write_str("[debug] ATA drives=");
    crate::drivers::serial::write_hex(ata.ata_devices as usize);
    crate::drivers::serial::write_str(" ATAPI=");
    crate::drivers::serial::write_hex(ata.atapi_devices as usize);
    crate::drivers::serial::write_str("\n");
}
