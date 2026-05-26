#[derive(Clone, Copy)]
pub struct VfsFile {
    pub name: &'static str,
    pub data: &'static [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VfsError {
    InvalidPath,
    InvalidDisk,
    NotFound,
    NotUtf8,
    WriteFailed,
    Unsupported,
}

pub type VfsResult<T> = Result<T, VfsError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VfsBackend {
    Root,
    Ramfs,
    Fat32,
}

#[derive(Clone, Copy)]
pub struct ParsedPath<'a> {
    pub backend: VfsBackend,
    pub path: &'a str,
}

pub fn error_str(error: VfsError) -> &'static str {
    match error {
        VfsError::InvalidPath => "Invalid path",
        VfsError::InvalidDisk => "Invalid disk",
        VfsError::NotFound => "File not found",
        VfsError::NotUtf8 => "File is not valid UTF-8",
        VfsError::WriteFailed => "Write failed",
        VfsError::Unsupported => "Unsupported operation",
    }
}

pub fn normalize_path(path: &str) -> &str {
    let mut p = path;

    while p.starts_with('/') {
        p = &p[1..];
    }

    p
}

pub fn parse_path(path: &str) -> VfsResult<ParsedPath<'_>> {
    if path.is_empty() {
        return Err(VfsError::InvalidPath);
    }

    if path == "/" {
        return Ok(ParsedPath {
            backend: VfsBackend::Root,
            path: "",
        });
    }

    let path = normalize_path(path);

    if path.is_empty() {
        return Ok(ParsedPath {
            backend: VfsBackend::Root,
            path: "",
        });
    }

    if path == "ram" {
        return Ok(ParsedPath {
            backend: VfsBackend::Ramfs,
            path: "",
        });
    }

    if path.starts_with("ram/") {
        return Ok(ParsedPath {
            backend: VfsBackend::Ramfs,
            path: &path[4..],
        });
    }

    if path == "disk1" {
        return Ok(ParsedPath {
            backend: VfsBackend::Fat32,
            path: "",
        });
    }

    if path.starts_with("disk1/") {
        return Ok(ParsedPath {
            backend: VfsBackend::Fat32,
            path: &path[6..],
        });
    }
    
    /*
        Default backend for /file.txt.

        You can choose:
        - Ramfs as root
        - or root as virtual directory only

        For now I'm setting /file.txt -> RAMFS.
    */
    Ok(ParsedPath {
        backend: VfsBackend::Ramfs,
        path,
    })
}

pub fn read(path: &str) -> VfsResult<&'static [u8]> {
    let parsed = parse_path(path)?;

    match parsed.backend {
        VfsBackend::Root => Err(VfsError::Unsupported),

        VfsBackend::Ramfs => {
            let relative = normalize_path(parsed.path);

            crate::fs::ramfs::read(relative.as_bytes())
                .ok_or(VfsError::NotFound)
        }

        VfsBackend::Fat32 => {
            crate::drivers::serial::write_str("[vfs] FAT32 read, mounted=");

            if crate::fs::fat32::is_mounted() {
                crate::drivers::serial::write_str("true\n");
            } else {
                crate::drivers::serial::write_str("false\n");
                return Err(VfsError::InvalidDisk);
            }

            let relative = normalize_path(parsed.path);

            crate::drivers::serial::write_str("[vfs] FAT32 path='");
            crate::drivers::serial::write_str(relative);
            crate::drivers::serial::write_str("'\n");

            if relative.is_empty() {
                return Err(VfsError::InvalidPath);
            }

            crate::fs::fat32::read_file(relative)
        }
    }
}

pub fn write(path: &str, data: &[u8]) -> VfsResult<()> {
    let parsed = parse_path(path)?;

    match parsed.backend {
        VfsBackend::Root => Err(VfsError::Unsupported),

        VfsBackend::Ramfs => {
            let relative = normalize_path(parsed.path);

            if crate::fs::ramfs::write(relative.as_bytes(), data) {
                Ok(())
            } else {
                Err(VfsError::WriteFailed)
            }
        }

        /*
            For now block write support on FAT32, as it's more complex and we don't have a real use case for it yet.
        */
        VfsBackend::Fat32 => Err(VfsError::WriteFailed),
    }
}

pub fn delete(path: &str) -> VfsResult<()> {
    let parsed = parse_path(path)?;

    match parsed.backend {
        VfsBackend::Root => Err(VfsError::Unsupported),

        VfsBackend::Ramfs => {
            let relative = normalize_path(parsed.path);

            if crate::fs::ramfs::delete(relative.as_bytes()) {
                Ok(())
            } else {
                Err(VfsError::NotFound)
            }
        }

        VfsBackend::Fat32 => Err(VfsError::WriteFailed),
    }
}

pub fn exists(path: &str) -> bool {
    read(path).is_ok()
}

pub fn disk_name(backend: VfsBackend) -> &'static str {
    match backend {
        VfsBackend::Root => "root",
        VfsBackend::Ramfs => "ramfs",
        VfsBackend::Fat32 => "fat32",
    }
}