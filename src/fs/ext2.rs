use crate::fs::vfs::VfsError;

const FILE_READ_BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MiB
static mut FILE_READ_BUFFER: [u8; FILE_READ_BUFFER_SIZE] = [0u8; FILE_READ_BUFFER_SIZE];

// Store mounted image as raw pointer + length to avoid creating shared references
static mut MOUNTED_PTR: usize = 0;
static mut MOUNTED_LEN: usize = 0;

fn read_u16(data: &[u8], off: usize) -> u16 {
    (data[off] as u16) | ((data[off + 1] as u16) << 8)
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    (data[off] as u32)
        | ((data[off + 1] as u32) << 8)
        | ((data[off + 2] as u32) << 16)
        | ((data[off + 3] as u32) << 24)
}

fn mounted_len_bytes() -> Option<usize> {
    unsafe {
        let ptr = core::ptr::addr_of!(MOUNTED_LEN);
        let len = *ptr;
        if len == 0 { None } else { Some(len) }
    }
}

fn read_bytes(offset: usize, out: &mut [u8]) -> bool {
    if out.is_empty() { return true; }
    let total = match mounted_len_bytes() { Some(v) => v, None => return false };
    if offset > total || out.len() > total.saturating_sub(offset) { return false; }
    unsafe {
        let base = *core::ptr::addr_of!(MOUNTED_PTR);
        let total = *core::ptr::addr_of!(MOUNTED_LEN);
        let src = core::slice::from_raw_parts(base as *const u8, total);
        out.copy_from_slice(&src[offset..offset + out.len()]);
    }
    true
}

pub fn mount(data: &'static [u8]) -> bool {
    // very small probe: check superblock magic at 1024+56 == 0xEF53
    if data.len() < 2048 { return false; }
    let magic = read_u16(data, 1024 + 56);
    if magic != 0xEF53 { return false; }

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
    unsafe {
        let ptr = core::ptr::addr_of!(MOUNTED_PTR);
        *ptr != 0
    }
}

fn parse_superblock(data: &[u8]) -> Option<(usize, usize, usize)> {
    // return (block_size, inode_size, inode_table_block)
    if data.len() < 2048 { return None; }
    let s_log_block_size = read_u32(data, 1024 + 24) as usize;
    let block_size = 1024usize << s_log_block_size;

    let inode_size = read_u16(data, 1024 + 88) as usize;
    let inode_size = if inode_size == 0 { 128 } else { inode_size };

    // read group descriptor 0 at block: if block_size == 1024, gd at 2048 else at block_size
    let gd_off = if block_size == 1024 { 2048 } else { block_size };
    if data.len() < gd_off + 12 { return None; }
    let inode_table = read_u32(data, gd_off + 8) as usize;

    Some((block_size, inode_size, inode_table))
}

fn read_inode(inode_no: u32, block_size: usize, inode_size: usize, inode_table_block: usize) -> Option<(u16, u32, [u32; 15])> {
    // return (mode, size, blocks)
    let it_off = inode_table_block * block_size;
    let ino = inode_no as usize;
    let off = it_off + (ino - 1) * inode_size;
    // inode_size is typically <= 256; use fixed buffer
    let mut buf_small = [0u8; 256];
    let buf_slice = &mut buf_small[..inode_size.min(256)];
    if !read_bytes(off, buf_slice) { return None; }

    let mode = read_u16(buf_slice, 0);
    let size = read_u32(buf_slice, 4);
    let mut blocks = [0u32; 15];
    for i in 0..15 {
        let boff = 40 + i * 4;
        if boff + 4 <= buf_slice.len() {
            blocks[i] = read_u32(buf_slice, boff);
        }
    }

    Some((mode, size, blocks))
}

pub fn read_file(path: &str) -> Result<&'static [u8], VfsError> {
    let data = unsafe {
        let base = *core::ptr::addr_of!(MOUNTED_PTR);
        let len = *core::ptr::addr_of!(MOUNTED_LEN);
        if base == 0 { return Err(VfsError::InvalidDisk); }
        core::slice::from_raw_parts(base as *const u8, len)
    };

    // only support files in root (no subdirs)
    if path.contains('/') { return Err(VfsError::NotFound); }

    let (block_size, inode_size, inode_table_block) = parse_superblock(data).ok_or(VfsError::InvalidDisk)?;

    // read root inode (2)
    let (_mode, _size, root_blocks) = read_inode(2, block_size, inode_size, inode_table_block).ok_or(VfsError::InvalidDisk)?;

    // scan directory entries in root blocks
    let mut found_inode: Option<u32> = None;
    // block buffer up to 4096
    static mut BLOCK_BUF: [u8; 4096] = [0u8; 4096];

    for &b in root_blocks.iter() {
        if b == 0 { continue; }
        let off = (b as usize) * block_size;
        let buf_len = block_size.min(4096);
        let buf = unsafe { &mut BLOCK_BUF[..buf_len] };
        if !read_bytes(off, buf) { continue; }
        let mut idx = 0usize;
        while idx + 8 < buf.len() {
            let inode = read_u32(buf, idx);
            if inode == 0 { break; }
            let rec_len = read_u16(buf, idx + 4) as usize;
            let name_len = buf[idx + 6] as usize;
            if name_len > 0 && idx + 8 + name_len <= buf.len() {
                let name = core::str::from_utf8(&buf[idx + 8..idx + 8 + name_len]).unwrap_or("");
                if name.eq_ignore_ascii_case(path) {
                    found_inode = Some(inode);
                    break;
                }
            }
            if rec_len == 0 { break; }
            idx += rec_len;
        }
        if found_inode.is_some() { break; }
    }

    let inode_no = found_inode.ok_or(VfsError::NotFound)?;
    let (_mode, size, blocks) = read_inode(inode_no, block_size, inode_size, inode_table_block).ok_or(VfsError::InvalidDisk)?;

    if size as usize > FILE_READ_BUFFER_SIZE { return Err(VfsError::InvalidDisk); }

    let dest: &mut [u8] = unsafe { &mut FILE_READ_BUFFER[..(size as usize)] };
    let mut written = 0usize;

    for &b in blocks.iter() {
        if b == 0 { break; }
        let off = (b as usize) * block_size;
        let to_copy = core::cmp::min(block_size, dest.len() - written);
        if !read_bytes(off, &mut dest[written..written + to_copy]) { return Err(VfsError::InvalidDisk); }
        written += to_copy;
        if written >= dest.len() { break; }
    }

    if written < dest.len() { return Err(VfsError::InvalidDisk); }

    Ok(dest)
}

pub fn print_dir(path: &str) -> Result<(), VfsError> {
    let data = unsafe {
        let base = *core::ptr::addr_of!(MOUNTED_PTR);
        let len = *core::ptr::addr_of!(MOUNTED_LEN);
        if base == 0 { return Err(VfsError::InvalidDisk); }
        core::slice::from_raw_parts(base as *const u8, len)
    };
    if !path.is_empty() { return Err(VfsError::Unsupported); }

    let (block_size, inode_size, inode_table_block) = parse_superblock(data).ok_or(VfsError::InvalidDisk)?;
    let (_mode, _size, root_blocks) = read_inode(2, block_size, inode_size, inode_table_block).ok_or(VfsError::InvalidDisk)?;

    static mut BLOCK_BUF: [u8; 4096] = [0u8; 4096];

    for &b in root_blocks.iter() {
        if b == 0 { continue; }
        let off = (b as usize) * block_size;
        let buf_len = block_size.min(4096);
        let buf = unsafe { &mut BLOCK_BUF[..buf_len] };
        if !read_bytes(off, buf) { continue; }

        let mut idx = 0usize;
        while idx + 8 < buf.len() {
            let inode = read_u32(buf, idx);
            if inode == 0 { break; }
            let rec_len = read_u16(buf, idx + 4) as usize;
            let name_len = buf[idx + 6] as usize;
            if name_len > 0 && idx + 8 + name_len <= buf.len() {
                let name = core::str::from_utf8(&buf[idx + 8..idx + 8 + name_len]).unwrap_or("");
                crate::kernel::write_raw(name);
                crate::print!("\n");
            }
            if rec_len == 0 { break; }
            idx += rec_len;
        }
    }

    Ok(())
}
