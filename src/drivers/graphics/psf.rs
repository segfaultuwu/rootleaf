#[repr(C)]
struct Psf2Header {
    magic: [u8; 4],
    version: u32,
    header_size: u32,
    flags: u32,
    glyph_count: u32,
    char_size: u32,
    height: u32,
    width: u32,
}

pub struct Psf2 {
    data: &'static [u8],
    header_size: u32,
    glyph_count: u32,
    char_size: u32,
    height: u32,
    width: u32,
}

impl Psf2 {
    pub fn new(data: &'static [u8]) -> Option<Self> {
        if data.len() < core::mem::size_of::<Psf2Header>() {
            return None;
        }

        let header = unsafe {
            core::ptr::read_unaligned(data.as_ptr() as *const Psf2Header)
        };

        if !Self::is_valid_header(&header) {
            return None;
        }

        let header_size = header.header_size as usize;
        let glyph_bytes = header.glyph_count as usize * header.char_size as usize;

        if data.len() < header_size.saturating_add(glyph_bytes) {
            return None;
        }

        Some(Self {
            data,
            header_size: header.header_size,
            glyph_count: header.glyph_count,
            char_size: header.char_size,
            height: header.height,
            width: header.width,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::is_valid_parts(self.header_size, self.glyph_count, self.char_size, self.height, self.width)
    }

    pub fn glyphs(&self) -> &[u8] {
        let start = self.header_size as usize;
        let end = start + (self.glyph_count as usize * self.char_size as usize);
        &self.data[start..end]
    }

    pub fn char_size(&self) -> u32 {
        self.char_size
    }

    pub fn char_count(&self) -> u32 {
        self.glyph_count
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    fn is_valid_header(header: &Psf2Header) -> bool {
        header.magic == [0x72, 0xB5, 0x4A, 0x86] && header.version == 0
    }

    fn is_valid_parts(
        header_size: u32,
        glyph_count: u32,
        char_size: u32,
        _height: u32,
        _width: u32,
    ) -> bool {
        header_size >= core::mem::size_of::<Psf2Header>() as u32 && glyph_count > 0 && char_size > 0
    }
}