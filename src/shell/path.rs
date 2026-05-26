pub fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

pub fn clear_line_buffer(line: &mut [u8; 128]) {
    for byte in line.iter_mut() {
        *byte = 0;
    }
}

pub fn eq_ignore_ascii_case(a: &[u8], b: &[u8]) -> bool {
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

pub fn starts_with_ignore_ascii_case(a: &[u8], prefix: &[u8]) -> bool {
    if a.len() < prefix.len() {
        return false;
    }

    eq_ignore_ascii_case(&a[..prefix.len()], prefix)
}

pub fn to_ascii_upper(byte: u8) -> u8 {
    if byte >= b'a' && byte <= b'z' {
        byte - 32
    } else {
        byte
    }
}

pub fn make_absolute_path<'a>(input: &'a [u8], buffer: &'a mut [u8; 128]) -> Option<&'a str> {
    let input = trim_ascii(input);

    if input.is_empty() {
        return None;
    }

    if looks_absolute_path(input) {
        return core::str::from_utf8(input).ok();
    }

    let prefix = crate::fs::cwd::get().as_bytes();

    let mut len = 0usize;

    for &b in prefix {
        if len >= buffer.len() {
            return None;
        }

        buffer[len] = b;
        len += 1;
    }

    for &b in input {
        if len >= buffer.len() {
            return None;
        }

        buffer[len] = b;
        len += 1;
    }

    core::str::from_utf8(&buffer[..len]).ok()
}

pub fn looks_absolute_path(input: &[u8]) -> bool {
    let mut i = 0usize;

    while i < input.len() {
        if input[i] == b':' {
            return i + 1 < input.len() && input[i + 1] == b'\\';
        }

        if input[i] < b'0' || input[i] > b'9' {
            return false;
        }

        i += 1;
    }

    false
}
