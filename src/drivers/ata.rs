use crate::arch::x86_64::port;

const ATA_SECTOR_SIZE: usize = 512;

#[derive(Clone, Copy)]
struct AtaChannel {
    io: u16,
    ctrl: u16,
}

#[derive(Clone, Copy)]
struct AtaDrive {
    channel: AtaChannel,
    drive_index: u8,
    sectors: u32,
}

static mut FIRST_DRIVE: Option<AtaDrive> = None;

fn wait_not_busy(io: u16) -> bool {
    let mut spin = 100_000usize;

    while spin > 0 {
        let status = unsafe { port::inb(io + 7) };

        if status == 0xFF {
            return false;
        }

        if (status & 0x80) == 0 {
            return true;
        }

        spin -= 1;
    }

    false
}

fn identify_drive(channel: AtaChannel, drive_index: u8) -> Option<AtaDrive> {
    let io = channel.io;
    let select = if drive_index == 0 { 0xA0 } else { 0xB0 };

    if !wait_not_busy(io) {
        return None;
    }

    unsafe {
        port::outb(io + 6, select);
        for _ in 0..4 {
            port::io_wait();
        }

        port::outb(io + 2, 0);
        port::outb(io + 3, 0);
        port::outb(io + 4, 0);
        port::outb(io + 5, 0);
        port::outb(io + 7, 0xEC);
    }

    let mut status = unsafe { port::inb(io + 7) };
    if status == 0 || status == 0xFF {
        return None;
    }

    let mut spin = 100_000usize;
    while spin > 0 {
        status = unsafe { port::inb(io + 7) };

        if (status & 0x01) != 0 || (status & 0x20) != 0 {
            return None;
        }

        if (status & 0x80) == 0 {
            break;
        }

        spin -= 1;
    }

    let lba_mid = unsafe { port::inb(io + 4) };
    let lba_hi = unsafe { port::inb(io + 5) };

    if lba_mid != 0 || lba_hi != 0 {
        return None;
    }

    spin = 100_000;
    while spin > 0 {
        status = unsafe { port::inb(io + 7) };

        if (status & 0x01) != 0 || (status & 0x20) != 0 {
            return None;
        }

        if (status & 0x08) != 0 {
            break;
        }

        spin -= 1;
    }

    if spin == 0 {
        return None;
    }

    let mut identify = [0u16; 256];
    for word in identify.iter_mut() {
        *word = unsafe { port::inw(io) };
    }

    let sectors = (identify[60] as u32) | ((identify[61] as u32) << 16);

    if sectors == 0 {
        return None;
    }

    Some(AtaDrive {
        channel,
        drive_index,
        sectors,
    })
}

fn first_ata_drive() -> Option<AtaDrive> {
    let channels = [
        AtaChannel { io: 0x1F0, ctrl: 0x3F6 },
        AtaChannel { io: 0x170, ctrl: 0x376 },
    ];

    for channel in channels {
        if let Some(drive) = identify_drive(channel, 0) {
            return Some(drive);
        }

        if let Some(drive) = identify_drive(channel, 1) {
            return Some(drive);
        }
    }

    None
}

fn get_first_drive() -> Option<AtaDrive> {
    unsafe {
        if let Some(d) = FIRST_DRIVE {
            return Some(d);
        }
    }

    let detected = first_ata_drive();

    unsafe {
        FIRST_DRIVE = detected;
    }

    detected
}

fn read_sector_lba28(drive: AtaDrive, lba: u32, dst: &mut [u8]) -> bool {
    if dst.len() < ATA_SECTOR_SIZE {
        return false;
    }

    if lba > 0x0FFF_FFFF {
        return false;
    }

    let io = drive.channel.io;
    let ctrl = drive.channel.ctrl;
    let head = ((lba >> 24) & 0x0F) as u8;

    if !wait_not_busy(io) {
        return false;
    }

    unsafe {
        port::outb(io + 6, 0xE0 | ((drive.drive_index & 1) << 4) | head);
        for _ in 0..4 {
            port::inb(ctrl);
        }

        port::outb(io + 2, 1);
        port::outb(io + 3, (lba & 0xFF) as u8);
        port::outb(io + 4, ((lba >> 8) & 0xFF) as u8);
        port::outb(io + 5, ((lba >> 16) & 0xFF) as u8);
        port::outb(io + 7, 0x20);
    }

    let mut spin = 100_000usize;
    while spin > 0 {
        let status = unsafe { port::inb(io + 7) };

        if (status & 0x01) != 0 || (status & 0x20) != 0 {
            return false;
        }

        if (status & 0x08) != 0 {
            break;
        }

        spin -= 1;
    }

    if spin == 0 {
        return false;
    }

    for i in 0..256usize {
        let w = unsafe { port::inw(io) };
        dst[i * 2] = (w & 0xFF) as u8;
        dst[i * 2 + 1] = (w >> 8) as u8;
    }

    unsafe {
        for _ in 0..4 {
            port::inb(ctrl);
        }
    }

    true
}

pub fn load_first_disk_image() -> Result<&'static [u8], &'static str> {
    Err("Use sector-based ATA reads; full disk preload disabled")
}

pub fn first_disk_sectors() -> Option<u32> {
    get_first_drive().map(|d| d.sectors)
}

pub fn read_first_sector(lba: u32, dst: &mut [u8; ATA_SECTOR_SIZE]) -> Result<(), &'static str> {
    let drive = match get_first_drive() {
        Some(d) => d,
        None => return Err("No ATA disk found"),
    };

    if lba >= drive.sectors {
        return Err("LBA out of range");
    }

    if !read_sector_lba28(drive, lba, dst) {
        return Err("ATA read failed");
    }

    Ok(())
}