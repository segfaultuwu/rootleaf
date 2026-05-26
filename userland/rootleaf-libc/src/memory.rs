#![allow(clippy::missing_safety_doc)]

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0usize;

    while i < n {
        unsafe {
            *dest.add(i) = *src.add(i);
        }

        i += 1;
    }

    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dest: *mut u8, value: i32, n: usize) -> *mut u8 {
    let byte = value as u8;
    let mut i = 0usize;

    while i < n {
        unsafe {
            *dest.add(i) = byte;
        }

        i += 1;
    }

    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if dest as usize <= src as usize {
        let mut i = 0usize;

        while i < n {
            unsafe {
                *dest.add(i) = *src.add(i);
            }

            i += 1;
        }
    } else {
        let mut i = n;

        while i > 0 {
            i -= 1;

            unsafe {
                *dest.add(i) = *src.add(i);
            }
        }
    }

    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0usize;

    while i < n {
        let av = unsafe { *a.add(i) };
        let bv = unsafe { *b.add(i) };

        if av != bv {
            return av as i32 - bv as i32;
        }

        i += 1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    unsafe { memcmp(a, b, n) }
}