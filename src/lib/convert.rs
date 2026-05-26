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
