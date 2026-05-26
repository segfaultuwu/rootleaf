pub fn u32_to_str(mut value: u32, buf: &mut [u8; 10]) -> &str {
    if value == 0 {
        buf[0] = b'0';
        return unsafe { core::str::from_utf8_unchecked(&buf[..1]) };
    }

    let mut i = buf.len();

    while value > 0 {
        i -= 1;
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    unsafe { core::str::from_utf8_unchecked(&buf[i..]) }
}

pub fn u64_to_str(mut value: u64, buffer: &mut [u8]) -> &str {
    if buffer.is_empty() {
        return "";
    }

    if value == 0 {
        buffer[0] = b'0';
        return unsafe { core::str::from_utf8_unchecked(&buffer[..1]) };
    }

    let mut temp = [0u8; 20];
    let mut len = 0usize;

    while value > 0 {
        temp[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }

    let out_len = len.min(buffer.len());

    for i in 0..out_len {
        buffer[i] = temp[len - 1 - i];
    }

    unsafe { core::str::from_utf8_unchecked(&buffer[..out_len]) }
}

pub fn u64_to_hex(mut value: u64, buffer: &mut [u8]) -> &str {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    if buffer.len() < 3 {
        return "";
    }

    buffer[0] = b'0';
    buffer[1] = b'x';

    if value == 0 {
        buffer[2] = b'0';
        return unsafe { core::str::from_utf8_unchecked(&buffer[..3]) };
    }

    let mut temp = [0u8; 16];
    let mut len = 0usize;

    while value > 0 {
        temp[len] = HEX[(value & 0xF) as usize];
        value >>= 4;
        len += 1;
    }

    let max_digits = buffer.len().saturating_sub(2);
    let digits = len.min(max_digits);

    for i in 0..digits {
        buffer[2 + i] = temp[len - 1 - i];
    }

    unsafe { core::str::from_utf8_unchecked(&buffer[..2 + digits]) }
}
