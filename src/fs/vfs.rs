#[derive(Clone, Copy)]
pub struct VfsFile {
    pub name: &'static str,
    pub data: &'static [u8],
}

#[derive(Clone, Copy)]
pub struct VfsPath<'a> {
    pub disk: usize,
    pub path: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VfsError {
    InvalidPath,
    InvalidDisk,
    NotFound,
    NotUtf8,
}

pub type VfsResult<T> = Result<T, VfsError>;

pub fn parse_path(path: &str) -> VfsResult<VfsPath<'_>> {
    let bytes = path.as_bytes();

    if bytes.len() < 3 {
        return Err(VfsError::InvalidPath);
    }

    let mut disk = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        let b = bytes[i];

        if b == b':' {
            break;
        }

        if b < b'0' || b > b'9' {
            return Err(VfsError::InvalidPath);
        }

        disk = disk.saturating_mul(10).saturating_add((b - b'0') as usize);

        i += 1;
    }

    if i >= bytes.len() || bytes[i] != b':' {
        return Err(VfsError::InvalidPath);
    }

    if i + 1 >= bytes.len() || bytes[i + 1] != b'\\' {
        return Err(VfsError::InvalidPath);
    }

    let rest = &path[i + 2..];

    Ok(VfsPath { disk, path: rest })
}

pub fn normalize_relative_path(path: &str) -> &str {
    let mut p = path;

    while p.starts_with('\\') {
        p = &p[1..];
    }

    p
}

pub fn read(path: &str) -> VfsResult<&'static [u8]> {
    let parsed = parse_path(path)?;

    match parsed.disk {
        0 => {
            let relative = normalize_relative_path(parsed.path);

            match crate::fs::ramfs::find(relative.as_bytes()) {
                Some(file) => Ok(file.data),
                None => Err(VfsError::NotFound),
            }
        }

        _ => Err(VfsError::InvalidDisk),
    }
}

pub fn exists(path: &str) -> bool {
    read(path).is_ok()
}

pub fn list(path: &str) -> VfsResult<&'static [crate::fs::ramfs::RamFile]> {
    let parsed = parse_path(path)?;

    match parsed.disk {
        0 => Ok(crate::fs::ramfs::files()),

        _ => Err(VfsError::InvalidDisk),
    }
}

pub fn disk_name(disk: usize) -> &'static str {
    match disk {
        0 => "RAMFS",
        _ => "UNKNOWN",
    }
}

pub fn error_str(error: VfsError) -> &'static str {
    match error {
        VfsError::InvalidPath => "Invalid path",
        VfsError::InvalidDisk => "Invalid disk",
        VfsError::NotFound => "File not found",
        VfsError::NotUtf8 => "File is not valid UTF-8",
    }
}
