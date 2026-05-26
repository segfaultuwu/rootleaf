pub mod ahci;
pub mod ata;
pub mod graphics;
pub mod keyboard;
pub mod pci;
pub mod sata;
pub mod serial;

pub fn init() {
    serial::init();

    crate::drivers::serial::write_str("Rootleaf: serial initialized\n");

    ahci::init();

    keyboard::init();
    crate::drivers::serial::write_str("Rootleaf: keyboard initialized\n");
}
