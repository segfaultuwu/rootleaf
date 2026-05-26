use core::cell::UnsafeCell;

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
    Isofs,
    Dev,
    Proc,
    Ext2,
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

const MAX_MOUNTS: usize = 8;
const MAX_MOUNT_NAME: usize = 32;

#[derive(Clone, Copy)]
struct MountEntry {
    used: bool,
    name: [u8; MAX_MOUNT_NAME],
    len: usize,
    backend: VfsBackend,
}

impl MountEntry {
    const fn empty() -> Self {
        Self {
            used: false,
            name: [0; MAX_MOUNT_NAME],
            len: 0,
            backend: VfsBackend::Ramfs,
        }
    }
}

struct MountTable(UnsafeCell<[MountEntry; MAX_MOUNTS]>);
unsafe impl Sync for MountTable {}

static MOUNTS: MountTable = MountTable(UnsafeCell::new([MountEntry::empty(); MAX_MOUNTS]));

pub fn mount(name: &str, backend: VfsBackend) -> bool {
    let name = normalize_path(name);
    if name.is_empty() || name.len() >= MAX_MOUNT_NAME {
        return false;
    }

    unsafe {
        let mounts = &mut *MOUNTS.0.get();

        // Reject duplicates
        for m in mounts.iter_mut() {
            if m.used && eq_ascii_ignore_case_str(name, core::str::from_utf8(&m.name[..m.len]).unwrap_or("")) {
                m.backend = backend;
                return true;
            }
        }

        for m in mounts.iter_mut() {
            if !m.used {
                m.used = true;
                m.len = name.len();
                for (i, &b) in name.as_bytes().iter().enumerate() {
                    m.name[i] = b;
                }
                m.backend = backend;
                return true;
            }
        }
    }

    false
}

pub fn unmount(name: &str) -> bool {
    let name = normalize_path(name);
    unsafe {
        let mounts = &mut *MOUNTS.0.get();
        for m in mounts.iter_mut() {
            if m.used && eq_ascii_ignore_case_str(name, core::str::from_utf8(&m.name[..m.len]).unwrap_or("")) {
                m.used = false;
                m.len = 0;
                return true;
            }
        }
    }

    false
}

pub fn parse_path(path: &str) -> VfsResult<VfsPath<'_>> {
    if path.is_empty() {
        return Err(VfsError::InvalidPath);
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

    // Check dynamic mounts first
    unsafe {
        let mounts = &*MOUNTS.0.get();
        // exact match
        for m in mounts.iter() {
            if !m.used { continue; }
            let mname = core::str::from_utf8(&m.name[..m.len]).unwrap_or("");
            if eq_ascii_ignore_case_str(path, mname) {
                return Ok(VfsPath { backend: m.backend, path: "" });
            }
        }
        // prefix match
        for m in mounts.iter() {
            if !m.used { continue; }
            let mname = core::str::from_utf8(&m.name[..m.len]).unwrap_or("");
            if starts_with_mount(path, mname) {
                return Ok(VfsPath { backend: m.backend, path: &path[mname.len()+1..] });
            }
        }
    }

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

pub fn normalize_path(path: &str) -> &str {
    let mut p = path;

    while p.starts_with('/') || p.starts_with('\\') {
        p = &p[1..];
    }

    p
}

pub fn read(path: &str) -> VfsResult<&'static [u8]> {
    // special-case /etc/mtab to expose the current mount table
    let norm = normalize_path(path);
    if norm == "etc/mtab" {
        return Ok(build_mtab());
    }

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
        VfsBackend::Isofs => {
            let relative = normalize_path(parsed.path);

            if relative.is_empty() {
                return Err(VfsError::InvalidPath);
            }

            crate::fs::isofs::read_file(relative)
        }
        VfsBackend::Ext2 => {
            let relative = normalize_path(parsed.path);

            if relative.is_empty() {
                return Err(VfsError::InvalidPath);
            }

            match crate::fs::ext2::read_file(relative) {
                Ok(data) => Ok(data),
                Err(e) => Err(e),
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
        
        VfsBackend::Fat32 => Err(VfsError::WriteFailed),
        VfsBackend::Isofs => Err(VfsError::WriteFailed),
        VfsBackend::Ext2 => Err(VfsError::WriteFailed),

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
        VfsBackend::Isofs => Err(VfsError::Unsupported),
        VfsBackend::Ext2 => Err(VfsError::Unsupported),
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
        VfsBackend::Isofs => "isofs",
        VfsBackend::Dev => "devfs",
        VfsBackend::Proc => "procfs",
        VfsBackend::Ext2 => "ext2",
    }
}

fn mount_device_prefix(backend: VfsBackend) -> &'static str {
    match backend {
        VfsBackend::Isofs => "cdrom",
        VfsBackend::Root => "root",
        VfsBackend::Dev => "dev",
        VfsBackend::Proc => "proc",
        VfsBackend::Ramfs => "loop",
        VfsBackend::Fat32 => "loop",
        VfsBackend::Ext2 => "loop",
    }
}

pub fn build_dev_list() -> &'static [u8] {
    static mut BUF: [u8; 512] = [0; 512];

    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 512] as *mut u8;
        let mut len = 0usize;

        // Always include a few virtual devices
        write_str(buf_ptr, 512, &mut len, "null\n");
        write_str(buf_ptr, 512, &mut len, "zero\n");
        write_str(buf_ptr, 512, &mut len, "console\n");
        write_str(buf_ptr, 512, &mut len, "tty\n");
        write_str(buf_ptr, 512, &mut len, "keyboard\n");

        // For each active mount entry, assign a synthetic block device name.
        // ISO9660 mounts are presented as CD-ROMs; everything else is loopback-backed.
        let mounts = &*MOUNTS.0.get();
        let mut mount_index = 0usize;
        for m in mounts.iter() {
            if !m.used { continue; }
            if matches!(m.backend, VfsBackend::Dev | VfsBackend::Proc | VfsBackend::Root) {
                continue;
            }

            write_str(buf_ptr, 512, &mut len, mount_device_prefix(m.backend));
            write_u64(buf_ptr, 512, &mut len, mount_index as u64);
            if len < 512 { core::ptr::write(buf_ptr.add(len), b'\n'); len += 1; }

            write_str(buf_ptr, 512, &mut len, mount_device_prefix(m.backend));
            write_u64(buf_ptr, 512, &mut len, mount_index as u64);
            write_str(buf_ptr, 512, &mut len, "p1");
            if len < 512 { core::ptr::write(buf_ptr.add(len), b'\n'); len += 1; }

            mount_index += 1;
        }

        // Detect physical disks and append unmounted disks if needed
        let st = crate::drivers::pci::scan_storage();
        let ata = crate::drivers::pci::scan_legacy_ata();
        let detected = st.total() + ata.ata_devices;

        let mut disk_index = 0usize;
        while disk_index < detected && disk_index < 26 {
            let letter = b'a' + (disk_index as u8);
            let nameb = [b's', b'd', letter];
            let s = core::str::from_utf8(&nameb).unwrap_or("sd?");
            write_str(buf_ptr, 512, &mut len, s);
            if len < 512 { core::ptr::write(buf_ptr.add(len), b'\n'); len += 1; }

            let partb = [b's', b'd', letter, b'1'];
            let ps = core::str::from_utf8(&partb).unwrap_or("sd?1");
            write_str(buf_ptr, 512, &mut len, ps);
            if len < 512 { core::ptr::write(buf_ptr.add(len), b'\n'); len += 1; }

            disk_index += 1;
        }


        core::slice::from_raw_parts(buf_ptr as *const u8, len)
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
        "" => Ok(build_dev_list()),
        "null" => Ok(b""),
        "zero" => Ok(b"\0\0\0\0\0\0\0\0"),
        "console" | "tty" => Ok(b"Rootleaf framebuffer console\n"),
        "keyboard" => Ok(b"PS/2 keyboard\n"),
            p if p.starts_with("cdrom") => Ok(b"CD-ROM drive (virtual)\n"),
        p if p.starts_with("sd") => {
            // simple device info for sd* and sd*n
            Ok(b"SCSI disk\n")
        }
        _ => Err(VfsError::NotFound),
    }
}

fn read_proc(path: &str) -> VfsResult<&'static [u8]> {
    static mut PROC_BUF: [u8; 512] = [0; 512];

    match normalize_path(path) {
        "" => Err(VfsError::Unsupported),
        "version" => Ok(concat_static(&[
            b"Rootleaf ",
            env!("CARGO_PKG_VERSION").as_bytes(),
            b"\n",
        ])),
        "cpuinfo" => Ok(build_cpuinfo()),
        "cwd" => Ok(build_cwd()),
        "pid" => Ok(build_pid()),
        "tasks" => Ok(build_tasks()),
        "mounts" => {
            Ok(build_mounts())
        }
        "meminfo" => Ok(build_meminfo()),
        "self" => {
            let pid = crate::scheduler::current_task_id();
            let _ = pid;
            Ok(build_self())
        }
        _ => Err(VfsError::NotFound),
    }
}

fn concat_static(parts: &[&[u8]]) -> &'static [u8] {
    static mut BUF: [u8; 128] = [0; 128];

    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 128] as *mut u8;
        let mut len = 0usize;

        for part in parts {
            for &byte in *part {
                if len >= 128 {
                    return core::slice::from_raw_parts(buf_ptr as *const u8, len);
                }

                core::ptr::write(buf_ptr.add(len), byte);
                len += 1;
            }
        }

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn write_str(out_ptr: *mut u8, max: usize, len: &mut usize, s: &str) {
    for &byte in s.as_bytes() {
        if *len >= max {
            return;
        }

        unsafe { core::ptr::write(out_ptr.add(*len), byte) };
        *len += 1;
    }
}

fn write_u64(out_ptr: *mut u8, max: usize, len: &mut usize, value: u64) {
    let mut num_buf = [0u8; 20];
    let text = crate::lib::u64_to_str(value, &mut num_buf);
    write_str(out_ptr, max, len, text);
}

fn build_cpuinfo() -> &'static [u8] {
    static mut BUF: [u8; 256] = [0; 256];
    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 256] as *mut u8;
        let mut len = 0usize;

        write_str(buf_ptr, 256, &mut len, "processor\t: 0\n");
        write_str(buf_ptr, 256, &mut len, "vendor_id\t: ");
        write_str(buf_ptr, 256, &mut len, crate::arch::x86_64::cpu::get_cpu_vendor());
        write_str(buf_ptr, 256, &mut len, "\nmodel name\t: x86_64\n");

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn build_cwd() -> &'static [u8] {
    static mut BUF: [u8; 128] = [0; 128];
    unsafe {
        let cwd = crate::fs::cwd::get().as_bytes();
        let buf_ptr = (&raw mut BUF) as *mut [u8; 128] as *mut u8;
        let mut len = 0usize;

        for &byte in cwd {
            if len >= 128 {
                break;
            }
            core::ptr::write(buf_ptr.add(len), byte);
            len += 1;
        }

        if len < 128 {
            core::ptr::write(buf_ptr.add(len), b'\n');
            len += 1;
        }

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn build_pid() -> &'static [u8] {
    static mut BUF: [u8; 32] = [0; 32];
    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 32] as *mut u8;
        let mut len = 0usize;

        write_u64(buf_ptr, 32, &mut len, crate::scheduler::current_task_id() as u64);
        if len < 32 {
            core::ptr::write(buf_ptr.add(len), b'\n');
            len += 1;
        }

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn build_tasks() -> &'static [u8] {
    static mut BUF: [u8; 32] = [0; 32];
    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 32] as *mut u8;
        let mut len = 0usize;

        write_u64(buf_ptr, 32, &mut len, crate::scheduler::task_count() as u64);
        if len < 32 {
            core::ptr::write(buf_ptr.add(len), b'\n');
            len += 1;
        }

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn build_mounts() -> &'static [u8] {
    static mut BUF: [u8; 128] = [0; 128];
    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 128] as *mut u8;
        let mut len = 0usize;

        write_str(buf_ptr, 128, &mut len, "rootfs /\nramfs /ram\n");

        if crate::fs::fat32::is_mounted() {
            write_str(buf_ptr, 128, &mut len, "fat32 /disk1\n");
        }

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn build_meminfo() -> &'static [u8] {
    static mut BUF: [u8; 256] = [0; 256];
    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 256] as *mut u8;
        let mut len = 0usize;
        let info = crate::memory::info::memory_info();

        write_str(buf_ptr, 256, &mut len, "MemTotal: ");
        write_u64(buf_ptr, 256, &mut len, info.total_mib());
        write_str(buf_ptr, 256, &mut len, " MiB\nMemFree:  ");
        write_u64(buf_ptr, 256, &mut len, info.usable_mib());
        write_str(buf_ptr, 256, &mut len, " MiB\nMemUsed:  ");
        write_u64(
            buf_ptr,
            256,
            &mut len,
            info.total_mib().saturating_sub(info.usable_mib()),
        );
        write_str(buf_ptr, 256, &mut len, " MiB\n");

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn build_self() -> &'static [u8] {
    static mut BUF: [u8; 32] = [0; 32];

    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 32] as *mut u8;
        let mut len = 0usize;

        write_str(buf_ptr, 32, &mut len, "self\n");
        core::slice::from_raw_parts(buf_ptr as *const u8, len)
    }
}

fn build_mtab() -> &'static [u8] {
    static mut BUF: [u8; 512] = [0; 512];

    unsafe {
        let buf_ptr = (&raw mut BUF) as *mut [u8; 512] as *mut u8;
        let mut len = 0usize;

        let mounts = &*MOUNTS.0.get();
        let mut mount_index = 0usize;
        for m in mounts.iter() {
            if !m.used { continue; }

            let mname = core::str::from_utf8(&m.name[..m.len]).unwrap_or("");
            write_str(buf_ptr, 512, &mut len, mount_device_prefix(m.backend));
            write_u64(buf_ptr, 512, &mut len, mount_index as u64);
            // Format similar to mtab: "<device> /<mount> <fstype> rw 0 0\n"
            if len < 512 {
                core::ptr::write(buf_ptr.add(len), b' ');
                len += 1;
            }
            write_str(buf_ptr, 512, &mut len, "/");
            write_str(buf_ptr, 512, &mut len, mname);
            if len < 512 {
                core::ptr::write(buf_ptr.add(len), b' ');
                len += 1;
            }
            write_str(buf_ptr, 512, &mut len, disk_name(m.backend));
            write_str(buf_ptr, 512, &mut len, " rw 0 0\n");

            mount_index += 1;
        }

        core::slice::from_raw_parts(buf_ptr as *const u8, len)
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