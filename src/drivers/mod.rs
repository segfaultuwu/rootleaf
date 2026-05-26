pub mod ahci;
pub mod ata;
pub mod graphics;
pub mod keyboard;
pub mod pci;
pub mod serial;
pub mod sata;

pub fn init() {
    serial::init();
    sata::init();
    keyboard::init();
}