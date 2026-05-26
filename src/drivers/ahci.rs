use crate::drivers::pci;

pub fn detect() -> usize {
    // Return number of SATA controllers found
    pci::scan_for_sata()
}
