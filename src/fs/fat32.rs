use crate::fs::vfs::VfsError;

const FILE_READ_BUFFER_SIZE: usize = 2 * 1024 * 1024;
static mut FILE_READ_BUFFER: [u8; FILE_READ_BUFFER_SIZE] = [0u8; FILE_READ_BUFFER_SIZE];

#[derive(Clone, Copy)]
enum MountedDisk {
    Memory(&'static [u8]),
    Ata { sectors: u32 },
}

static mut MOUNTED: Option<MountedDisk> = None;

#[derive(Clone, Copy)]
struct Fat32Meta {
    bps: usize,
    spc: usize,
    reserved: usize,
    fats: usize,
    fatsz: usize,
    root_cluster: u32,
    first_data_sector: u32,
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    (data[off] as u16) | ((data[off + 1] as u16) << 8)
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    (data[off] as u32)
        | ((data[off + 1] as u32) << 8)
        | ((data[off + 2] as u32) << 16)
        | ((data[off + 3] as u32) << 24)
}

pub fn mount(data: &'static [u8]) -> bool {
    unsafe {
        MOUNTED = Some(MountedDisk::Memory(data));
    }

    true
}

pub fn mount_first_ata() -> Result<(), &'static str> {
    let sectors = match crate::drivers::ata::first_disk_sectors() {
        Some(s) if s > 0 => s,
        _ => return Err("No ATA disk found"),
    };

    unsafe {
        MOUNTED = Some(MountedDisk::Ata { sectors });
    }

    Ok(())
}

pub fn unmount() {
    unsafe {
        MOUNTED = None;
    }
}

pub fn is_mounted() -> bool {
    unsafe { MOUNTED.is_some() }
}

fn mounted() -> Option<&'static [u8]> {
    None
}

fn mounted_disk() -> Option<MountedDisk> {
    unsafe { MOUNTED }
}

fn mounted_len_bytes() -> Option<usize> {
    match mounted_disk()? {
        MountedDisk::Memory(data) => Some(data.len()),
        MountedDisk::Ata { sectors } => Some((sectors as usize).saturating_mul(512)),
    }
}

fn read_bytes(offset: usize, out: &mut [u8]) -> bool {
    if out.is_empty() {
        return true;
    }

    let total = match mounted_len_bytes() {
        Some(v) => v,
        None => return false,
    };

    if offset > total || out.len() > total.saturating_sub(offset) {
        return false;
    }

    match mounted_disk() {
        Some(MountedDisk::Memory(data)) => {
            out.copy_from_slice(&data[offset..offset + out.len()]);
            true
        }
        Some(MountedDisk::Ata { .. }) => {
            let mut done = 0usize;

            while done < out.len() {
                let abs = offset + done;
                let lba = (abs / 512) as u32;
                let sector_off = abs % 512;

                let mut sector = [0u8; 512];
                if crate::drivers::ata::read_first_sector(lba, &mut sector).is_err() {
                    return false;
                }

                let n = core::cmp::min(512 - sector_off, out.len() - done);
                out[done..done + n].copy_from_slice(&sector[sector_off..sector_off + n]);
                done += n;
            }

            true
        }
        None => false,
    }
}

fn read_u16_at(off: usize) -> Option<u16> {
    let mut b = [0u8; 2];
    if !read_bytes(off, &mut b) {
        return None;
    }
    Some(read_u16(&b, 0))
}

fn read_u32_at(off: usize) -> Option<u32> {
    let mut b = [0u8; 4];
    if !read_bytes(off, &mut b) {
        return None;
    }
    Some(read_u32(&b, 0))
}

fn parse_meta() -> Option<Fat32Meta> {
    let mut boot = [0u8; 512];
    if !read_bytes(0, &mut boot) {
        return None;
    }

    let bps = read_u16(&boot, 11) as usize;
    let spc = boot[13] as usize;
    let reserved = read_u16(&boot, 14) as usize;
    let fats = boot[16] as usize;
    let fatsz = read_u32(&boot, 36) as usize;
    let root_cluster = read_u32(&boot, 44);

    if bps == 0 || spc == 0 || reserved == 0 || fats == 0 || fatsz == 0 || root_cluster < 2 {
        return None;
    }

    let first_data_sector = reserved as u32 + (fats as u32 * fatsz as u32);

    Some(Fat32Meta {
        bps,
        spc,
        reserved,
        fats,
        fatsz,
        root_cluster,
        first_data_sector,
    })
}

fn cluster_to_offset(meta: Fat32Meta, cluster: u32) -> usize {
    let sector = meta.first_data_sector + (cluster - 2) * meta.spc as u32;
    (sector as usize) * meta.bps
}

fn read_fat_next(meta: Fat32Meta, cluster: u32) -> Option<u32> {
    let fat_offset = meta.reserved * meta.bps + (cluster as usize * 4);
    Some(read_u32_at(fat_offset)? & 0x0FFF_FFFF)
}

pub fn print_dir(path: &str) -> Result<(), VfsError> {
    let _ = path;

    let meta = match parse_meta() {
        Some(m) => m,
        None => return Err(VfsError::InvalidDisk),
    };

    // iterate directory starting at root_cluster
    let mut cluster = meta.root_cluster;

    crate::kernel::write_raw(" Directory listing:\n");

    loop {
        let cluster_bytes = meta.spc * meta.bps;
        let base = cluster_to_offset(meta, cluster);

        for i in (0..cluster_bytes).step_by(32) {
            let mut entry = [0u8; 32];
            if !read_bytes(base + i, &mut entry) {
                return Ok(());
            }

            let first = entry[0];
            if first == 0x00 {
                // end of directory
                return Ok(());
            }

            if first == 0xE5 || entry[11] == 0x0F {
                continue;
            }

            // short name
            let name_raw = &entry[0..11];

            let mut name = [0u8; 13];
            let mut ni = 0usize;

            for &b in &name_raw[0..8] {
                if b == b' ' {
                    break;
                }
                name[ni] = b;
                ni += 1;
            }

            if name_raw[8] != b' ' {
                name[ni] = b'.';
                ni += 1;

                for &b in &name_raw[8..11] {
                    if b == b' ' {
                        break;
                    }
                    name[ni] = b;
                    ni += 1;
                }
            }

            let s = core::str::from_utf8(&name[..ni]).unwrap_or("?");
            crate::kernel::write_raw("  ");
            crate::kernel::write_raw(s);
            crate::print!("\n");
        }

        // read next cluster from FAT
        let next = match read_fat_next(meta, cluster) {
            Some(v) => v,
            None => break,
        };

        if next >= 0x0FFFFFF8 || next == 0 {
            break;
        }

        cluster = next;
    }

    Ok(())
}

pub fn read_file(path: &str) -> Result<&'static [u8], VfsError> {
    let meta = match parse_meta() {
        Some(m) => m,
        None => return Err(VfsError::InvalidDisk),
    };

    // split path (only support single filename in root for now)
    let p = path.trim_start_matches('\\');

    // simple search in root dir
    let mut cluster = meta.root_cluster;
    let mut found_start: Option<u32> = None;
    let mut found_size: usize = 0;

    loop {
        let cluster_bytes = meta.spc * meta.bps;
        let off = cluster_to_offset(meta, cluster);

        for i in (0..cluster_bytes).step_by(32) {
            let mut entry = [0u8; 32];
            if !read_bytes(off + i, &mut entry) {
                break;
            }

            let first = entry[0];
            if first == 0x00 { break; }
            if first == 0xE5 || entry[11] == 0x0F { continue; }

            // short name
            let name_raw = &entry[0..11];
            let mut name_str = [0u8; 13];
            let mut ni = 0usize;

            for &b in &name_raw[0..8] {
                if b == b' ' { break; }
                name_str[ni] = b; ni += 1;
            }
            if name_raw[8] != b' ' {
                name_str[ni] = b'.'; ni += 1;
                for &b in &name_raw[8..11] {
                    if b == b' ' { break; }
                    name_str[ni] = b; ni += 1;
                }
            }

            let s = core::str::from_utf8(&name_str[..ni]).unwrap_or("");
            if s.eq_ignore_ascii_case(p) {
                let high = read_u16(&entry, 20) as u32;
                let low = read_u16(&entry, 26) as u32;
                let start = (high << 16) | low;
                let size = read_u32(&entry, 28) as usize;
                found_start = Some(start);
                found_size = size;
                break;
            }
        }

        if found_start.is_some() { break; }

        let next = match read_fat_next(meta, cluster) {
            Some(v) => v,
            None => break,
        };
        if next >= 0x0FFFFFF8 || next == 0 { break; }
        cluster = next;
    }

    let start = match found_start { Some(s) => s, None => return Err(VfsError::NotFound) };

    if found_size > FILE_READ_BUFFER_SIZE {
        return Err(VfsError::InvalidDisk);
    }

    let dest: &mut [u8] = unsafe { &mut FILE_READ_BUFFER[..found_size] };

    // read cluster chain
    let mut out_written = 0usize;
    let mut cur = start;

    loop {
        if out_written >= found_size { break; }

        let off = cluster_to_offset(meta, cur);

        let to_copy = core::cmp::min(meta.spc * meta.bps, found_size - out_written);
        if !read_bytes(off, &mut dest[out_written..out_written + to_copy]) {
            return Err(VfsError::InvalidDisk);
        }
        out_written += to_copy;

        // next cluster
        let next = match read_fat_next(meta, cur) {
            Some(v) => v,
            None => break,
        };
        if next >= 0x0FFFFFF8 || next == 0 { break; }
        cur = next;
    }

    if out_written < found_size {
        return Err(VfsError::InvalidDisk);
    }

    Ok(dest)
}
