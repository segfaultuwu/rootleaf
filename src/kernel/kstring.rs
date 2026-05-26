use core::fmt;

pub struct KString<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> KString<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.len]) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn push_byte(&mut self, byte: u8) -> Result<(), fmt::Error> {
        if self.len >= N {
            return Err(fmt::Error);
        }

        self.buf[self.len] = byte;
        self.len += 1;

        Ok(())
    }
}

impl<const N: usize> fmt::Write for KString<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            self.push_byte(b)?;
        }

        Ok(())
    }
}

pub fn format_args_to_kstring<const N: usize>(args: fmt::Arguments) -> KString<N> {
    use core::fmt::Write;

    let mut s = KString::<N>::new();
    let _ = s.write_fmt(args);
    s
}
