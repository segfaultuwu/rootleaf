#[derive(Clone, Copy)]
pub struct VfsFile {
    pub name: &'static str,
    pub data: &'static [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VfsBackend {
    Root,
    Ramfs,
    Fat32,
    Dev,
    Proc,
}

#[derive(Clone, Copy)]
pub struct VfsPath<'a> {
    pub backend: VfsBackend,
    pub path: &'a str,
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

pub fn parse_path(path: &str) -> VfsResult<VfsPath<'_>> {
    if path.is_empty() {
        return Err(VfsError::InvalidPath);
    }

    /*
        Legacy compatibility:
            0:\FILE -> /ram/FILE
            1:\FILE -> /disk1/FILE
    */
    if is_legacy_path(path) {
        return parse_legacy_path(path);
    }

    if path == "/" {
        return Ok(VfsPath {
            backend: VfsBackend::Root,
            path: "",
        });
    }

    if !path.starts_with('/') {
        return Err(VfsError::InvalidPath);
    }

    let path = normalize_path(path);

    if path.is_empty() {
        return Ok(VfsPath {
            backend: VfsBackend::Root,
            path: "",
        });
    }

    if eq_ascii_ignore_case_str(path, "ram") {
        return Ok(VfsPath {
            backend: VfsBackend::Ramfs,
            path: "",
        });
    }

    if starts_with_mount(path, "ram") {
        return Ok(VfsPath {
            backend: VfsBackend::Ramfs,
            path: &path[4..],
        });
    }

    if eq_ascii_ignore_case_str(path, "disk1") {
        return Ok(VfsPath {
            backend: VfsBackend::Fat32,
            path: "",
        });
    }

    if starts_with_mount(path, "disk1") {
        return Ok(VfsPath {
            backend: VfsBackend::Fat32,
            path: &path[6..],
        });
    }

    if eq_ascii_ignore_case_str(path, "dev") {
        return Ok(VfsPath {
            backend: VfsBackend::Dev,
            path: "",
        });
    }

    if starts_with_mount(path, "dev") {
        return Ok(VfsPath {
            backend: VfsBackend::Dev,
            path: &path[4..],
        });
    }

    if eq_ascii_ignore_case_str(path, "proc") {
        return Ok(VfsPath {
            backend: VfsBackend::Proc,
            path: "",
        });
    }

    if starts_with_mount(path, "proc") {
        return Ok(VfsPath {
            backend: VfsBackend::Proc,
            path: &path[5..],
        });
    }

    /*
        Default:
            /file.txt -> RAMFS file
    */
    Ok(VfsPath {
        backend: VfsBackend::Ramfs,
        path,
    })
}

fn is_legacy_path(path: &str) -> bool {
    let bytes = path.as_bytes();

    bytes.len() >= 2 && bytes[1] == b':'
}

fn parse_legacy_path(path: &str) -> VfsResult<VfsPath<'_>> {
    let bytes = path.as_bytes();

    if bytes.len() < 2 || bytes[1] != b':' {
        return Err(VfsError::InvalidPath);
    }

    let mut rest = if bytes.len() > 2 {
        &path[2..]
    } else {
        ""
    };

    while rest.starts_with('\\') || rest.starts_with('/') {
        rest = &rest[1..];
    }

    match bytes[0] {
        b'0' => Ok(VfsPath {
            backend: VfsBackend::Ramfs,
            path: rest,
        }),

        b'1' => Ok(VfsPath {
            backend: VfsBackend::Fat32,
            path: rest,
        }),

        _ => Err(VfsError::InvalidDisk),
    }
}

pub fn normalize_path(path: &str) -> &str {
    let mut p = path;

    while p.starts_with('/') || p.starts_with('\\') {
        p = &p[1..];
    }

    p
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

            match crate::fs::fat32::read_file(relative) {
                Ok(data) => Ok(data),

                Err(first_error) => {
                    let mut upper_buf = [0u8; 128];

                    let Some(upper) = uppercase_fat_path(relative, &mut upper_buf) else {
                        return Err(first_error);
                    };

                    crate::drivers::serial::write_str("[vfs] FAT32 retry uppercase path='");
                    crate::drivers::serial::write_str(upper);
                    crate::drivers::serial::write_str("'\n");

                    crate::fs::fat32::read_file(upper)
                }
            }
        }

        VfsBackend::Dev => read_dev(parsed.path),

        VfsBackend::Proc => read_proc(parsed.path),
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
            FAT32 write is disabled until your FAT32 write path is stable.
        */
        VfsBackend::Fat32 => Err(VfsError::WriteFailed),

        VfsBackend::Dev => Err(VfsError::Unsupported),

        VfsBackend::Proc => Err(VfsError::Unsupported),
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

        VfsBackend::Dev => Err(VfsError::Unsupported),

        VfsBackend::Proc => Err(VfsError::Unsupported),
    }
}

pub fn exists(path: &str) -> bool {
    read(path).is_ok()
}

pub fn disk_name(backend: VfsBackend) -> &'static str {
    match backend {
        VfsBackend::Root => "rootfs",
        VfsBackend::Ramfs => "ramfs",
        VfsBackend::Fat32 => "fat32",
        VfsBackend::Dev => "devfs",
        VfsBackend::Proc => "procfs",
    }
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

fn read_dev(path: &str) -> VfsResult<&'static [u8]> {
    match normalize_path(path) {
        "" => Err(VfsError::Unsupported),
        "null" => Ok(b""),
        "zero" => Ok(b"\0\0\0\0\0\0\0\0"),
        "console" => Ok(b"Rootleaf framebuffer console\n"),
        "keyboard" => Ok(b"PS/2 keyboard\n"),
        _ => Err(VfsError::NotFound),
    }
}

fn read_proc(path: &str) -> VfsResult<&'static [u8]> {
    match normalize_path(path) {
        "" => Err(VfsError::Unsupported),
        "version" => Ok(b"Rootleaf\n"),
        "cpuinfo" => Ok(b"x86_64\n"),
        "mounts" => {
            if crate::fs::fat32::is_mounted() {
                Ok(b"rootfs /\nramfs /ram\nfat32 /disk1\n")
            } else {
                Ok(b"rootfs /\nramfs /ram\n")
            }
        }
        _ => Err(VfsError::NotFound),
    }
}

fn uppercase_fat_path<'a>(input: &str, out: &'a mut [u8; 128]) -> Option<&'a str> {
    let bytes = input.as_bytes();

    if bytes.len() >= out.len() {
        return None;
    }

    for i in 0..bytes.len() {
        let b = bytes[i];

        out[i] = match b {
            b'a'..=b'z' => b - 32,
            b'/' => b'\\',
            _ => b,
        };
    }

    core::str::from_utf8(&out[..bytes.len()]).ok()
}

fn eq_ascii_ignore_case_str(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();

    if a.len() != b.len() {
        return false;
    }

    for i in 0..a.len() {
        if to_ascii_lower(a[i]) != to_ascii_lower(b[i]) {
            return false;
        }
    }

    true
}

fn starts_with_mount(path: &str, mount: &str) -> bool {
    let path_bytes = path.as_bytes();
    let mount_bytes = mount.as_bytes();

    if path_bytes.len() <= mount_bytes.len() {
        return false;
    }

    if path_bytes[mount_bytes.len()] != b'/' {
        return false;
    }

    for i in 0..mount_bytes.len() {
        if to_ascii_lower(path_bytes[i]) != to_ascii_lower(mount_bytes[i]) {
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