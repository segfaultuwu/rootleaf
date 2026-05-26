pub fn trim_ascii(mut input: &[u8]) -> &[u8] {
    while !input.is_empty() && input[0].is_ascii_whitespace() {
        input = &input[1..];
    }

    while !input.is_empty() && input[input.len() - 1].is_ascii_whitespace() {
        input = &input[..input.len() - 1];
    }

    input
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

pub fn make_absolute_path<'a>(input: &[u8], buffer: &'a mut [u8; 128]) -> Option<&'a str> {
    let input = trim_ascii(input);

    if input.is_empty() {
        return None;
    }

    let input_str = core::str::from_utf8(input).ok()?;

    if is_legacy_disk_path(input_str) {
        return legacy_to_linux_path(input_str, buffer);
    }

    let mut len = 0usize;

    if input_str.starts_with('/') {
        copy_bytes(input_str.as_bytes(), buffer, &mut len)?;
    } else {
        let cwd = crate::fs::cwd::get();

        copy_bytes(cwd.as_bytes(), buffer, &mut len)?;

        if len == 0 || buffer[len - 1] != b'/' {
            push_byte(buffer, &mut len, b'/')?;
        }

        copy_bytes(input_str.as_bytes(), buffer, &mut len)?;
    }

    normalize_slashes_in_place(buffer, &mut len);

    while len > 1 && buffer[len - 1] == b'/' {
        len -= 1;
    }

    core::str::from_utf8(&buffer[..len]).ok()
}

fn is_legacy_disk_path(path: &str) -> bool {
    let bytes = path.as_bytes();

    bytes.len() >= 2 && bytes[1] == b':'
}

fn legacy_to_linux_path<'a>(path: &str, buffer: &'a mut [u8; 128]) -> Option<&'a str> {
    let bytes = path.as_bytes();

    if bytes.len() < 2 || bytes[1] != b':' {
        return None;
    }

    let mut len = 0usize;

    match bytes[0] {
        b'0' => copy_bytes(b"/ram", buffer, &mut len)?,
        b'1' => copy_bytes(b"/disk1", buffer, &mut len)?,
        _ => return None,
    }

    if bytes.len() > 2 {
        let mut rest = &path[2..];

        while rest.starts_with('\\') || rest.starts_with('/') {
            rest = &rest[1..];
        }

        if !rest.is_empty() {
            push_byte(buffer, &mut len, b'/')?;
            copy_bytes(rest.as_bytes(), buffer, &mut len)?;
        }
    }

    normalize_slashes_in_place(buffer, &mut len);

    while len > 1 && buffer[len - 1] == b'/' {
        len -= 1;
    }

    core::str::from_utf8(&buffer[..len]).ok()
}

fn copy_bytes(bytes: &[u8], buffer: &mut [u8; 128], len: &mut usize) -> Option<()> {
    for &b in bytes {
        push_byte(buffer, len, b)?;
    }

    Some(())
}

fn push_byte(buffer: &mut [u8; 128], len: &mut usize, byte: u8) -> Option<()> {
    if *len >= buffer.len() {
        return None;
    }

    buffer[*len] = byte;
    *len += 1;

    Some(())
}

fn normalize_slashes_in_place(buffer: &mut [u8; 128], len: &mut usize) {
    for i in 0..*len {
        if buffer[i] == b'\\' {
            buffer[i] = b'/';
        }
    }

    let mut read = 0usize;
    let mut write = 0usize;
    let mut last_was_slash = false;

    while read < *len {
        let b = buffer[read];

        if b == b'/' {
            if !last_was_slash {
                buffer[write] = b;
                write += 1;
            }

            last_was_slash = true;
        } else {
            buffer[write] = b;
            write += 1;
            last_was_slash = false;
        }

        read += 1;
    }

    *len = write;
}