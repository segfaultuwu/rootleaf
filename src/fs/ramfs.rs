#[derive(Clone, Copy)]
pub struct RamFile {
    pub name: &'static str,
    pub data: &'static [u8],
}

static README: &[u8] = b"Welcome to Rootleaf RAMFS.\nThis file lives in kernel memory.\n";
static CONFIG: &[u8] = b"DEVICE=FRAMEBUFFER\nKEYBOARD=PS2\nSHELL=ROOTLEAF\n";
static AUTOEXEC: &[u8] = b"ECHO Rootleaf booted\n";

static FILES: &[RamFile] = &[
    RamFile {
        name: "README.TXT",
        data: README,
    },
    RamFile {
        name: "CONFIG.SYS",
        data: CONFIG,
    },
    RamFile {
        name: "AUTOEXEC.BAT",
        data: AUTOEXEC,
    },
];

const USER_MAX: usize = 32;
const USER_NAME_LEN: usize = 64;
const USER_DATA_LEN: usize = 8192;

#[derive(Clone, Copy)]
struct UserFile {
    used: bool,
    name: [u8; USER_NAME_LEN],
    name_len: usize,
    data: [u8; USER_DATA_LEN],
    data_len: usize,
}

static mut USER_FILES: [UserFile; USER_MAX] = [UserFile {
    used: false,
    name: [0; USER_NAME_LEN],
    name_len: 0,
    data: [0; USER_DATA_LEN],
    data_len: 0,
}; USER_MAX];

pub fn files() -> &'static [RamFile] {
    FILES
}

pub fn find(name: &[u8]) -> Option<&'static RamFile> {
    for file in FILES {
        if eq_ignore_ascii_case(name, file.name.as_bytes()) {
            return Some(file);
        }
    }

    None
}

fn find_user_slot(name: &[u8]) -> Option<usize> {
    unsafe {
        for i in 0..USER_MAX {
            if !USER_FILES[i].used {
                continue;
            }

            if eq_ignore_ascii_case(name, &USER_FILES[i].name[..USER_FILES[i].name_len]) {
                return Some(i);
            }
        }
    }

    None
}

pub fn read(name: &[u8]) -> Option<&'static [u8]> {
    if let Some(i) = find_user_slot(name) {
        unsafe {
            return Some(&USER_FILES[i].data[..USER_FILES[i].data_len]);
        }
    }

    find(name).map(|f| f.data)
}

pub fn write(name: &[u8], data: &[u8]) -> bool {
    if name.is_empty() || name.len() > USER_NAME_LEN || data.len() > USER_DATA_LEN {
        return false;
    }

    unsafe {
        if let Some(i) = find_user_slot(name) {
            USER_FILES[i].data[..data.len()].copy_from_slice(data);
            USER_FILES[i].data_len = data.len();
            return true;
        }

        for i in 0..USER_MAX {
            if USER_FILES[i].used {
                continue;
            }

            USER_FILES[i].used = true;
            USER_FILES[i].name.fill(0);
            USER_FILES[i].name[..name.len()].copy_from_slice(name);
            USER_FILES[i].name_len = name.len();
            USER_FILES[i].data[..data.len()].copy_from_slice(data);
            USER_FILES[i].data_len = data.len();
            return true;
        }
    }

    false
}

pub fn delete(name: &[u8]) -> bool {
    if let Some(i) = find_user_slot(name) {
        unsafe {
            USER_FILES[i].used = false;
            USER_FILES[i].name_len = 0;
            USER_FILES[i].data_len = 0;
        }
        return true;
    }

    false
}

pub fn print_dir() {
    for file in FILES {
        crate::kernel::write_raw("  ");
        crate::kernel::write_raw(file.name);
        crate::print!("\n");
    }

    unsafe {
        for i in 0..USER_MAX {
            if !USER_FILES[i].used {
                continue;
            }

            if let Ok(name) = core::str::from_utf8(&USER_FILES[i].name[..USER_FILES[i].name_len]) {
                crate::kernel::write_raw("  ");
                crate::kernel::write_raw(name);
                crate::print!("\n");
            }
        }
    }
}

pub fn count() -> usize {
    FILES.len()
}

pub fn total_size() -> usize {
    let mut size = 0usize;

    for file in FILES {
        size += file.data.len();
    }

    size
}

fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for i in 0..a.len() {
        if to_ascii_upper(a[i]) != to_ascii_upper(b[i]) {
            return false;
        }
    }

    true
}

fn to_ascii_upper(byte: u8) -> u8 {
    if byte >= b'a' && byte <= b'z' {
        byte - 32
    } else {
        byte
    }
}
