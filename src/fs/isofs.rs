use crate::fs::vfs::VfsError;

const SECTOR_SIZE: usize = 2048;
const FILE_READ_BUFFER_SIZE: usize = 4 * 1024 * 1024;

static mut FILE_READ_BUFFER: [u8; FILE_READ_BUFFER_SIZE] = [0u8; FILE_READ_BUFFER_SIZE];
static mut MOUNTED_PTR: usize = 0;
static mut MOUNTED_LEN: usize = 0;

#[derive(Clone, Copy)]
struct IsoRecord {
    extent: usize,
    size: usize,
    flags: u8,
}

fn mounted_slice() -> Option<&'static [u8]> {
    unsafe {
        let base = core::ptr::read(core::ptr::addr_of!(MOUNTED_PTR));
        let len = core::ptr::read(core::ptr::addr_of!(MOUNTED_LEN));

        if base == 0 || len == 0 {
            None
        } else {
            Some(core::slice::from_raw_parts(base as *const u8, len))
        }
    }
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    (data[offset] as u32)
        | ((data[offset + 1] as u32) << 8)
        | ((data[offset + 2] as u32) << 16)
        | ((data[offset + 3] as u32) << 24)
}

fn read_bytes(offset: usize, out: &mut [u8]) -> bool {
    let data = match mounted_slice() {
        Some(data) => data,
        None => return false,
    };

    if offset > data.len() || out.len() > data.len().saturating_sub(offset) {
        return false;
    }

    out.copy_from_slice(&data[offset..offset + out.len()]);
    true
}

fn root_record() -> Option<IsoRecord> {
    let data = mounted_slice()?;
    let vd_off = 16 * SECTOR_SIZE;
    if data.len() < vd_off + 2048 {
        return None;
    }

    if data[vd_off] != 1 || &data[vd_off + 1..vd_off + 6] != b"CD001" {
        return None;
    }

    let root_off = vd_off + 156;
    let rec_len = data[root_off] as usize;
    if rec_len < 34 || root_off + rec_len > data.len() {
        return None;
    }

    Some(IsoRecord {
        extent: read_u32_le(data, root_off + 2) as usize,
        size: read_u32_le(data, root_off + 10) as usize,
        flags: data[root_off + 25],
    })
}

pub fn mount(data: &'static [u8]) -> bool {
    if data.len() < 16 * SECTOR_SIZE + 2048 {
        return false;
    }

    let vd_off = 16 * SECTOR_SIZE;
    if data[vd_off] != 1 || &data[vd_off + 1..vd_off + 6] != b"CD001" {
        return false;
    }

    unsafe {
        MOUNTED_PTR = data.as_ptr() as usize;
        MOUNTED_LEN = data.len();
    }

    true
}

pub fn unmount() {
    unsafe {
        MOUNTED_PTR = 0;
        MOUNTED_LEN = 0;
    }
}

pub fn is_mounted() -> bool {
    unsafe { MOUNTED_PTR != 0 && MOUNTED_LEN != 0 }
}

fn is_directory(record: IsoRecord) -> bool {
    record.flags & 0x02 != 0
}

fn strip_version(name: &[u8]) -> &[u8] {
    match name.iter().position(|&b| b == b';') {
        Some(pos) => &name[..pos],
        None => name,
    }
}

fn names_match(entry_name: &[u8], query: &str) -> bool {
    let entry_name = strip_version(entry_name);
    let query = strip_version(query.as_bytes());

    if entry_name.len() != query.len() {
        return false;
    }

    for i in 0..entry_name.len() {
        if to_ascii_lower(entry_name[i]) != to_ascii_lower(query[i]) {
            return false;
        }
    }

    true
}

fn to_ascii_lower(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' {
        byte + 32
    } else {
        byte
    }
}

fn for_each_dir_entry<F>(dir: IsoRecord, mut visit: F) -> bool
where
    F: FnMut(&[u8], IsoRecord) -> bool,
{
    let data = match mounted_slice() {
        Some(data) => data,
        None => return false,
    };

    let dir_off = dir.extent.saturating_mul(SECTOR_SIZE);
    let dir_end = dir_off.saturating_add(dir.size);
    if dir_off >= data.len() || dir_end > data.len() {
        return false;
    }

    let mut offset = 0usize;

    while offset < dir.size {
        let pos = dir_off + offset;
        let rec_len = data[pos] as usize;

        if rec_len == 0 {
            offset = ((offset / SECTOR_SIZE) + 1) * SECTOR_SIZE;
            continue;
        }

        if rec_len < 34 || offset + rec_len > dir.size || pos + rec_len > data.len() {
            break;
        }

        let name_len = data[pos + 32] as usize;
        if name_len > 0 && pos + 33 + name_len <= pos + rec_len {
            let name = &data[pos + 33..pos + 33 + name_len];
            let entry = IsoRecord {
                extent: read_u32_le(data, pos + 2) as usize,
                size: read_u32_le(data, pos + 10) as usize,
                flags: data[pos + 25],
            };

            if !visit(name, entry) {
                return true;
            }
        }

        offset += rec_len;
    }

    true
}

fn find_in_dir(dir: IsoRecord, component: &str) -> Option<IsoRecord> {
    let mut found = None;

    let _ = for_each_dir_entry(dir, |name, entry| {
        if name == b"\0" || name == b"\x01" {
            return true;
        }

        if names_match(name, component) {
            found = Some(entry);
            return false;
        }

        true
    });

    found
}

fn resolve_path(path: &str) -> Option<IsoRecord> {
    let mut current = root_record()?;
    let path = crate::fs::vfs::normalize_path(path);

    if path.is_empty() {
        return Some(current);
    }

    for component in path.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }

        current = find_in_dir(current, component)?;
    }

    Some(current)
}

pub fn read_file(path: &str) -> Result<&'static [u8], VfsError> {
    let record = resolve_path(path).ok_or(VfsError::NotFound)?;

    if is_directory(record) {
        return Err(VfsError::Unsupported);
    }

    let data = mounted_slice().ok_or(VfsError::InvalidDisk)?;
    let off = record.extent.saturating_mul(SECTOR_SIZE);
    let size = record.size.min(FILE_READ_BUFFER_SIZE);

    if off > data.len() || size > data.len().saturating_sub(off) {
        return Err(VfsError::InvalidDisk);
    }

    unsafe {
        let dest = &mut FILE_READ_BUFFER[..size];
        dest.copy_from_slice(&data[off..off + size]);
        Ok(dest)
    }
}

pub fn print_dir(path: &str) -> Result<(), VfsError> {
    let dir = resolve_path(path).ok_or(VfsError::NotFound)?;

    if !is_directory(dir) {
        return Err(VfsError::Unsupported);
    }

    let mut utf8_error = false;
    let _ = for_each_dir_entry(dir, |name, _entry| {
        if name == b"\0" || name == b"\x01" {
            return true;
        }

        let printable = strip_version(name);
        match core::str::from_utf8(printable) {
            Ok(text) => {
                crate::kernel::write_raw(text);
                crate::print!("\n");
            }
            Err(_) => {
                utf8_error = true;
                return false;
            }
        }

        true
    });

    if utf8_error {
        return Err(VfsError::NotUtf8);
    }

    Ok(())
}
