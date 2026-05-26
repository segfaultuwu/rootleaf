use core::ptr::{read_volatile, write_volatile};

use crate::drivers::serial;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

const AHCI_CLASS_MASS_STORAGE: u8 = 0x01;
const AHCI_SUBCLASS_SATA: u8 = 0x06;
const AHCI_PROGIF_AHCI: u8 = 0x01;

const HBA_PORT_DET_PRESENT: u32 = 3;
const HBA_PORT_IPM_ACTIVE: u32 = 1;

const SATA_SIG_ATA: u32 = 0x0000_0101;
const SATA_SIG_ATAPI: u32 = 0xEB14_0101;
const SATA_SIG_SEMB: u32 = 0xC33C_0101;
const SATA_SIG_PM: u32 = 0x9669_0101;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AhciDeviceType {
    None,
    Sata,
    Satapi,
    Semb,
    PortMultiplier,
    Unknown,
}

#[derive(Clone, Copy)]
pub struct AhciController {
    pub abar: usize,
    pub ports_implemented: u32,
}

static mut CONTROLLER: Option<AhciController> = None;

pub fn init() {
    serial::write_str("[sata] scanning PCI for AHCI controller\n");

    match find_ahci_controller() {
        Some(abar) => {
            serial::write_str("[sata] AHCI controller found, ABAR=");
            serial::write_hex(abar);
            serial::write_str("\n");

            let hba = abar as *mut HbaMem;

            let pi = unsafe {
                read_volatile(core::ptr::addr_of!((*hba).pi))
            };

            unsafe {
                core::ptr::write(
                    core::ptr::addr_of_mut!(CONTROLLER),
                    Some(AhciController {
                        abar,
                        ports_implemented: pi,
                    }),
                );
            }

            serial::write_str("[sata] ports implemented=");
            serial::write_hex(pi as usize);
            serial::write_str("\n");

            probe_ports(hba);
        }

        None => {
            serial::write_str("[sata] no AHCI controller found\n");
        }
    }
}

pub fn is_available() -> bool {
    unsafe {
        let ptr = core::ptr::addr_of!(CONTROLLER);
        (*ptr).is_some()
    }
}

pub fn controller() -> Option<AhciController> {
    unsafe {
        core::ptr::read(core::ptr::addr_of!(CONTROLLER))
    }
}

fn find_ahci_controller() -> Option<usize> {
    for bus in 0u8..=255 {
        for slot in 0u8..32 {
            for func in 0u8..8 {
                let vendor = pci_read_u16(bus, slot, func, 0x00);

                if vendor == 0xFFFF {
                    if func == 0 {
                        break;
                    }

                    continue;
                }

                let class = pci_read_u8(bus, slot, func, 0x0B);
                let subclass = pci_read_u8(bus, slot, func, 0x0A);
                let prog_if = pci_read_u8(bus, slot, func, 0x09);

                if class == AHCI_CLASS_MASS_STORAGE
                    && subclass == AHCI_SUBCLASS_SATA
                    && prog_if == AHCI_PROGIF_AHCI
                {
                    serial::write_str("[sata] PCI AHCI at ");
                    print_pci_addr(bus, slot, func);
                    serial::write_str("\n");

                    let bar5 = pci_read_u32(bus, slot, func, 0x24);

                    /*
                        BAR5 is AHCI ABAR.
                        For memory BAR:
                            bits 0..3 are flags
                            address = BAR & !0xF
                    */
                    let abar = (bar5 & 0xFFFF_FFF0) as usize;

                    if abar == 0 {
                        serial::write_str("[sata] AHCI BAR5 is zero\n");
                        return None;
                    }

                    return Some(abar);
                }
            }
        }
    }

    None
}

fn probe_ports(hba: *mut HbaMem) {
    let pi = unsafe {
        read_volatile(core::ptr::addr_of!((*hba).pi))
    };

    for port_index in 0..32 {
        if pi & (1 << port_index) == 0 {
            continue;
        }

        let port = unsafe {
            core::ptr::addr_of_mut!((*hba).ports[port_index])
        };

        let dev_type = check_port_type(port);

        serial::write_str("[sata] port ");
        serial::write_hex(port_index);
        serial::write_str(": ");

        match dev_type {
            AhciDeviceType::None => {
                serial::write_str("empty\n");
            }

            AhciDeviceType::Sata => {
                serial::write_str("SATA drive\n");
            }

            AhciDeviceType::Satapi => {
                serial::write_str("SATAPI device\n");
            }

            AhciDeviceType::Semb => {
                serial::write_str("SEMB device\n");
            }

            AhciDeviceType::PortMultiplier => {
                serial::write_str("port multiplier\n");
            }

            AhciDeviceType::Unknown => {
                serial::write_str("unknown device\n");
            }
        }
    }
}

fn check_port_type(port: *mut HbaPort) -> AhciDeviceType {
    let ssts = unsafe {
        read_volatile(core::ptr::addr_of!((*port).ssts))
    };

    let det = ssts & 0x0F;
    let ipm = (ssts >> 8) & 0x0F;

    if det != HBA_PORT_DET_PRESENT {
        return AhciDeviceType::None;
    }

    if ipm != HBA_PORT_IPM_ACTIVE {
        return AhciDeviceType::None;
    }

    let sig = unsafe {
        read_volatile(core::ptr::addr_of!((*port).sig))
    };

    match sig {
        SATA_SIG_ATA => AhciDeviceType::Sata,
        SATA_SIG_ATAPI => AhciDeviceType::Satapi,
        SATA_SIG_SEMB => AhciDeviceType::Semb,
        SATA_SIG_PM => AhciDeviceType::PortMultiplier,
        _ => AhciDeviceType::Unknown,
    }
}

fn pci_config_address(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((slot as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC)
}

fn pci_read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = pci_config_address(bus, slot, func, offset);

    unsafe {
        crate::arch::x86_64::port::outl(PCI_CONFIG_ADDRESS, address);
        crate::arch::x86_64::port::inl(PCI_CONFIG_DATA)
    }
}

fn pci_read_u16(bus: u8, slot: u8, func: u8, offset: u8) -> u16 {
    let value = pci_read_u32(bus, slot, func, offset & 0xFC);
    let shift = ((offset & 2) * 8) as u32;

    ((value >> shift) & 0xFFFF) as u16
}

fn pci_read_u8(bus: u8, slot: u8, func: u8, offset: u8) -> u8 {
    let value = pci_read_u32(bus, slot, func, offset & 0xFC);
    let shift = ((offset & 3) * 8) as u32;

    ((value >> shift) & 0xFF) as u8
}

fn print_pci_addr(bus: u8, slot: u8, func: u8) {
    serial::write_str("bus=");
    serial::write_hex(bus as usize);
    serial::write_str(" slot=");
    serial::write_hex(slot as usize);
    serial::write_str(" func=");
    serial::write_hex(func as usize);
}