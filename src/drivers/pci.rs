use crate::arch::x86_64::port;

const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

fn make_addr(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let b = bus as u32;
    let d = dev as u32;
    let f = func as u32;
    let off = (offset as u32) & 0xFC;

    0x80000000u32 | (b << 16) | (d << 11) | (f << 8) | off
}

pub fn config_read_u32(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let addr = make_addr(bus, dev, func, offset);

    unsafe {
        port::outl(PCI_CONFIG_ADDR, addr);
        port::inl(PCI_CONFIG_DATA)
    }
}

pub fn scan_for_sata() -> usize {
    let mut count = 0usize;

    for bus in 0u8..=255u8 {
        for dev in 0u8..32u8 {
            for func in 0u8..8u8 {
                let v = config_read_u32(bus, dev, func, 0x00);
                let vendor = (v & 0xFFFF) as u16;

                if vendor == 0xFFFF {
                    // no device
                    continue;
                }

                let cls = config_read_u32(bus, dev, func, 0x08);
                let class = (cls >> 24) as u8;
                let subclass = ((cls >> 16) & 0xFF) as u8;
                let prog_if = ((cls >> 8) & 0xFF) as u8;

                // Class 0x01 = mass storage, subclass 0x06 = SATA
                if class == 0x01 && subclass == 0x06 {
                    count += 1;
                }
            }
        }
    }

    count
}

#[derive(Clone, Copy)]
pub struct StorageSummary {
    pub ide: usize,
    pub sata: usize,
    pub scsi: usize,
    pub nvme: usize,
    pub other: usize,
}

impl StorageSummary {
    pub const fn total(self) -> usize {
        self.ide + self.sata + self.scsi + self.nvme + self.other
    }
}

pub fn scan_storage() -> StorageSummary {
    let mut out = StorageSummary {
        ide: 0,
        sata: 0,
        scsi: 0,
        nvme: 0,
        other: 0,
    };

    for bus in 0u8..=255u8 {
        for dev in 0u8..32u8 {
            for func in 0u8..8u8 {
                let v = config_read_u32(bus, dev, func, 0x00);
                let vendor = (v & 0xFFFF) as u16;

                if vendor == 0xFFFF {
                    continue;
                }

                let cls = config_read_u32(bus, dev, func, 0x08);
                let class = (cls >> 24) as u8;
                let subclass = ((cls >> 16) & 0xFF) as u8;

                if class != 0x01 {
                    continue;
                }

                match subclass {
                    0x01 => out.ide += 1,
                    0x06 => out.sata += 1,
                    0x00 => out.scsi += 1,
                    0x08 => out.nvme += 1,
                    _ => out.other += 1,
                }
            }
        }
    }

    out
}

#[derive(Clone, Copy)]
pub struct LegacyAtaSummary {
    pub channels: usize,
    pub ata_devices: usize,
    pub atapi_devices: usize,
}

fn detect_ata_on_channel(io_base: u16, drive_select: u8) -> (bool, bool) {
    unsafe {
        port::outb(io_base + 6, drive_select);
        for _ in 0..4 {
            port::io_wait();
        }

        port::outb(io_base + 2, 0);
        port::outb(io_base + 3, 0);
        port::outb(io_base + 4, 0);
        port::outb(io_base + 5, 0);
        port::outb(io_base + 7, 0xEC);

        let mut status = port::inb(io_base + 7);
        if status == 0 {
            return (false, false);
        }

        let mut spin = 100_000usize;
        while (status & 0x80) != 0 && spin > 0 {
            status = port::inb(io_base + 7);
            spin -= 1;
        }

        let lba1 = port::inb(io_base + 4);
        let lba2 = port::inb(io_base + 5);

        if lba1 != 0 || lba2 != 0 {
            return (false, true);
        }

        spin = 100_000;
        while spin > 0 {
            status = port::inb(io_base + 7);

            if (status & 0x01) != 0 {
                return (false, false);
            }

            if (status & 0x08) != 0 {
                return (true, false);
            }

            spin -= 1;
        }
    }

    (false, false)
}

pub fn scan_legacy_ata() -> LegacyAtaSummary {
    let mut out = LegacyAtaSummary {
        channels: 0,
        ata_devices: 0,
        atapi_devices: 0,
    };

    let channels = [0x1F0u16, 0x170u16];
    let selectors = [0xA0u8, 0xB0u8];

    for io_base in channels {
        let mut channel_seen = false;
        let base_status = unsafe { port::inb(io_base + 7) };

        if base_status != 0xFF && base_status != 0x00 {
            channel_seen = true;
        }

        for selector in selectors {
            let (ata, atapi) = detect_ata_on_channel(io_base, selector);

            if ata {
                out.ata_devices += 1;
                channel_seen = true;
            }

            if atapi {
                out.atapi_devices += 1;
                channel_seen = true;
            }
        }

        if channel_seen {
            out.channels += 1;
        }
    }

    out
}
