use core::cell::UnsafeCell;

struct CwdSlot(UnsafeCell<[u8; 128]>);

unsafe impl Sync for CwdSlot {}

static CWD_BUF: CwdSlot = CwdSlot(UnsafeCell::new([0; 128]));
static mut CWD_LEN: usize = 0;

pub fn init(default: &str) {
    let _ = set(default);
}

pub fn get() -> &'static str {
    unsafe {
        let buf = &*CWD_BUF.0.get();
        let slice = &buf[..CWD_LEN];
        core::str::from_utf8_unchecked(slice)
    }
}

pub fn set(s: &str) -> bool {
    let bytes = s.as_bytes();

    if bytes.len() >= 128 {
        return false;
    }

    unsafe {
        let buf = &mut *CWD_BUF.0.get();
        for i in 0..bytes.len() {
            buf[i] = bytes[i];
        }
        // zero out remainder for cleanliness
        for i in bytes.len()..128 {
            buf[i] = 0;
        }

        CWD_LEN = bytes.len();
    }

    true
}
