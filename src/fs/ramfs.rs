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
