use core::sync::atomic::{AtomicUsize, Ordering};

use crate::drivers::serial;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

const AHCI_CLASS_MASS_STORAGE: u8 = 0x01;
const AHCI_SUBCLASS_SATA: u8 = 0x06;
const AHCI_PROGIF_AHCI: u8 = 0x01;

/*
    Keep this false until we implement proper PCI MMIO mapping.

    AHCI BAR5 is a physical MMIO address, for example:

        0xf0806000

    Do NOT dereference it directly in a higher-half kernel.
    HHDM/phys_to_virt may not map PCI MMIO on our setup.
*/
const ENABLE_AHCI_MMIO_PROBE: bool = false;

const HBA_PORT_DET_PRESENT: u32 = 3;
const HBA_PORT_IPM_ACTIVE: u32 = 1;
const SATA_SIG_ATA: u32 = 0x0000_0101;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SataError {
    NoController,
    NoDrive,
    MmioNotMapped,
    BufferTooSmall,
    Timeout,
    DeviceError,
}

#[derive(Clone, Copy)]
pub struct AhciController {
    pub abar_phys: usize,
    pub abar_virt: usize,
    pub ports_implemented: u32,
    pub first_sata_port: Option<usize>,
    pub drive_count: usize,
}

static CONTROLLER_COUNT: AtomicUsize = AtomicUsize::new(0);
static DRIVE_COUNT: AtomicUsize = AtomicUsize::new(0);

static mut CONTROLLER: Option<AhciController> = None;

#[repr(C)]
pub struct HbaMem {
    pub cap: u32,
    pub ghc: u32,
    pub is: u32,
    pub pi: u32,
    pub vs: u32,
    pub ccc_ctl: u32,
    pub ccc_pts: u32,
    pub em_loc: u32,
    pub em_ctl: u32,
    pub cap2: u32,
    pub bohc: u32,

    pub reserved: [u8; 0xA0 - 0x2C],
    pub vendor: [u8; 0x100 - 0xA0],

    pub ports: [HbaPort; 32],
}

#[repr(C)]
pub struct HbaPort {
    pub clb: u32,
    pub clbu: u32,
    pub fb: u32,
    pub fbu: u32,
    pub is: u32,
    pub ie: u32,
    pub cmd: u32,
    pub reserved0: u32,
    pub tfd: u32,
    pub sig: u32,
    pub ssts: u32,
    pub sctl: u32,
    pub serr: u32,
    pub sact: u32,
    pub ci: u32,
    pub sntf: u32,
    pub fbs: u32,
    pub reserved1: [u32; 11],
    pub vendor: [u32; 4],
}

pub fn init() {
    serial::write_str("[sata] scanning PCI for AHCI controller\n");

    let Some(abar_phys) = find_ahci_controller() else {
        CONTROLLER_COUNT.store(0, Ordering::SeqCst);
        DRIVE_COUNT.store(0, Ordering::SeqCst);

        unsafe {
            core::ptr::write(core::ptr::addr_of_mut!(CONTROLLER), None);
        }

        serial::write_str("[sata] no AHCI controller found\n");
        return;
    };

    CONTROLLER_COUNT.store(1, Ordering::SeqCst);

    serial::write_str("[sata] AHCI controller found, ABAR phys=");
    serial::write_hex(abar_phys);
    serial::write_str("\n");

    if !ENABLE_AHCI_MMIO_PROBE {
        serial::write_str("[sata] MMIO probe disabled; not touching AHCI BAR yet\n");
        serial::write_str("[sata] controller detected, drive_count=0 until MMIO mapping exists\n");

        unsafe {
            core::ptr::write(
                core::ptr::addr_of_mut!(CONTROLLER),
                Some(AhciController {
                    abar_phys,
                    abar_virt: 0,
                    ports_implemented: 0,
                    first_sata_port: None,
                    drive_count: 0,
                }),
            );
        }

        DRIVE_COUNT.store(0, Ordering::SeqCst);
        return;
    }

    /*
        This block is only safe after you add real MMIO mapping.

        phys_to_virt() through HHDM crashed on your setup because AHCI BAR is
        MMIO, not regular RAM.
    */
    let abar_virt = crate::memory::addr::phys_to_virt(abar_phys);

    serial::write_str("[sata] AHCI ABAR virt=");
    serial::write_hex(abar_virt);
    serial::write_str("\n");

    let hba = abar_virt as *mut HbaMem;

    serial::write_str("[sata] reading PI\n");

    let pi = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*hba).pi)) };

    serial::write_str("[sata] ports implemented=");
    serial::write_hex(pi as usize);
    serial::write_str("\n");

    let mut first_sata_port = None;
    let mut drives = 0usize;

    for port_index in 0..32usize {
        if pi & (1 << port_index) == 0 {
            continue;
        }

        let port = unsafe { core::ptr::addr_of_mut!((*hba).ports[port_index]) };

        if check_port_sata(port, port_index) {
            drives += 1;

            if first_sata_port.is_none() {
                first_sata_port = Some(port_index);
            }
        }
    }

    DRIVE_COUNT.store(drives, Ordering::SeqCst);

    unsafe {
        core::ptr::write(
            core::ptr::addr_of_mut!(CONTROLLER),
            Some(AhciController {
                abar_phys,
                abar_virt,
                ports_implemented: pi,
                first_sata_port,
                drive_count: drives,
            }),
        );
    }

    if drives == 0 {
        serial::write_str("[sata] no SATA drive found\n");
    }
}

pub fn controller_count() -> usize {
    CONTROLLER_COUNT.load(Ordering::SeqCst)
}

pub fn drive_count() -> usize {
    DRIVE_COUNT.load(Ordering::SeqCst)
}

pub fn is_available() -> bool {
    controller_count() > 0
}

pub fn controller() -> Option<AhciController> {
    unsafe { core::ptr::read(core::ptr::addr_of!(CONTROLLER)) }
}

/*
    Stub for now.

    This returns MmioNotMapped until you implement:
        - PCI MMIO BAR mapping
        - DMA-safe physical buffers
        - AHCI command list/FIS/PRDT setup
*/
pub fn read_sector(_lba: u64, out: &mut [u8]) -> Result<(), SataError> {
    if out.len() < 512 {
        return Err(SataError::BufferTooSmall);
    }

    if !is_available() {
        return Err(SataError::NoController);
    }

    if !ENABLE_AHCI_MMIO_PROBE {
        return Err(SataError::MmioNotMapped);
    }

    Err(SataError::NoDrive)
}

fn check_port_sata(port: *mut HbaPort, port_index: usize) -> bool {
    let ssts = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*port).ssts)) };

    let det = ssts & 0x0f;
    let ipm = (ssts >> 8) & 0x0f;

    serial::write_str("[sata] port ");
    serial::write_hex(port_index);
    serial::write_str(" ssts=");
    serial::write_hex(ssts as usize);
    serial::write_str(" det=");
    serial::write_hex(det as usize);
    serial::write_str(" ipm=");
    serial::write_hex(ipm as usize);
    serial::write_str("\n");

    if det != HBA_PORT_DET_PRESENT || ipm != HBA_PORT_IPM_ACTIVE {
        return false;
    }

    let sig = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*port).sig)) };

    serial::write_str("[sata] port ");
    serial::write_hex(port_index);
    serial::write_str(" sig=");
    serial::write_hex(sig as usize);
    serial::write_str("\n");

    if sig == SATA_SIG_ATA {
        serial::write_str("[sata] SATA drive on port ");
        serial::write_hex(port_index);
        serial::write_str("\n");
        true
    } else {
        false
    }
}

fn find_ahci_controller() -> Option<usize> {
    for bus in 0u8..=255 {
        for slot in 0u8..32 {
            for func in 0u8..8 {
                let vendor = pci_read_u16(bus, slot, func, 0x00);

                if vendor == 0xffff {
                    if func == 0 {
                        break;
                    }

                    continue;
                }

                let class = pci_read_u8(bus, slot, func, 0x0b);
                let subclass = pci_read_u8(bus, slot, func, 0x0a);
                let prog_if = pci_read_u8(bus, slot, func, 0x09);

                if class == AHCI_CLASS_MASS_STORAGE
                    && subclass == AHCI_SUBCLASS_SATA
                    && prog_if == AHCI_PROGIF_AHCI
                {
                    serial::write_str("[sata] PCI AHCI at ");
                    print_pci_addr(bus, slot, func);
                    serial::write_str("\n");

                    let bar5 = pci_read_u32(bus, slot, func, 0x24);
                    let abar = (bar5 & 0xffff_fff0) as usize;

                    serial::write_str("[sata] BAR5=");
                    serial::write_hex(bar5 as usize);
                    serial::write_str(" ABAR=");
                    serial::write_hex(abar);
                    serial::write_str("\n");

                    if abar == 0 {
                        return None;
                    }

                    return Some(abar);
                }
            }
        }
    }

    None
}

fn pci_config_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xfc)
}

fn pci_read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = pci_config_address(bus, slot, func, offset);

    unsafe {
        crate::arch::x86_64::port::outl(PCI_CONFIG_ADDRESS, address);
        crate::arch::x86_64::port::inl(PCI_CONFIG_DATA)
    }
}

fn pci_read_u16(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    let value = pci_read_u32(bus, slot, func, offset & 0xfc);
    let shift = ((offset & 2) * 8) as u32;

    ((value >> shift) & 0xffff) as u16
}

fn pci_read_u8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
    let value = pci_read_u32(bus, slot, func, offset & 0xfc);
    let shift = ((offset & 3) * 8) as u32;

    ((value >> shift) & 0xff) as u8
}

fn print_pci_addr(bus: u8, slot: u8, func: u8) {
    serial::write_str("bus=");
    serial::write_hex(bus as usize);
    serial::write_str(" slot=");
    serial::write_hex(slot as usize);
    serial::write_str(" func=");
    serial::write_hex(func as usize);
}