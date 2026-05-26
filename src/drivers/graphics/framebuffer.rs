use limine::framebuffer::Framebuffer;

pub struct FramebufferDriver {
    addr: *mut u8,
    size: usize,
    width: usize,
    height: usize,
    pitch: usize,
    bytes_per_pixel: usize,
}

impl FramebufferDriver {
    pub fn new(framebuffer: &Framebuffer) -> Self {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let pitch = framebuffer.pitch as usize;
        let bytes_per_pixel = (framebuffer.bpp / 8) as usize;

        Self {
            addr: framebuffer.address() as *mut u8,
            size: pitch * height,
            width,
            height,
            pitch,
            bytes_per_pixel,
        }
    }

    pub fn from_raw(
        addr: *mut u8,
        width: usize,
        height: usize,
        pitch: usize,
        bytes_per_pixel: usize,
    ) -> Self {
        Self {
            addr,
            size: pitch * height,
            width,
            height,
            pitch,
            bytes_per_pixel,
        }
    }

    pub fn addr(&self) -> *mut u8 {
        self.addr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pitch(&self) -> usize {
        self.pitch
    }

    pub fn bytes_per_pixel(&self) -> usize {
        self.bytes_per_pixel
    }

    pub fn clear(&mut self, color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }

        if self.bytes_per_pixel < 3 {
            return;
        }

        let offset = y * self.pitch + x * self.bytes_per_pixel;

        if offset + 2 >= self.size {
            return;
        }

        unsafe {
            // Limine/QEMU zwykle daje framebuffer w formacie BGRX.
            self.addr.add(offset + 0).write_volatile(color.b);
            self.addr.add(offset + 1).write_volatile(color.g);
            self.addr.add(offset + 2).write_volatile(color.r);

            if self.bytes_per_pixel >= 4 && offset + 3 < self.size {
                self.addr.add(offset + 3).write_volatile(color.a);
            }
        }
    }

    pub fn get_pixel_offset(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let offset = y * self.pitch + x * self.bytes_per_pixel;

        if offset + 2 >= self.size {
            return None;
        }

        Some(offset)
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }

        let max_x = core::cmp::min(x.saturating_add(w), self.width);
        let max_y = core::cmp::min(y.saturating_add(h), self.height);

        for py in y..max_y {
            for px in x..max_x {
                self.put_pixel(px, py, color);
            }
        }
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        if w == 0 || h == 0 {
            return;
        }

        let x2 = x.saturating_add(w - 1);
        let y2 = y.saturating_add(h - 1);

        for px in x..=x2 {
            self.put_pixel(px, y, color);
            self.put_pixel(px, y2, color);
        }

        for py in y..=y2 {
            self.put_pixel(x, py, color);
            self.put_pixel(x2, py, color);
        }
    }

    pub fn draw_horizontal_line(&mut self, x: usize, y: usize, len: usize, color: Color) {
        if y >= self.height {
            return;
        }

        let max_x = core::cmp::min(x.saturating_add(len), self.width);

        for px in x..max_x {
            self.put_pixel(px, y, color);
        }
    }

    pub fn draw_vertical_line(&mut self, x: usize, y: usize, len: usize, color: Color) {
        if x >= self.width {
            return;
        }

        let max_y = core::cmp::min(y.saturating_add(len), self.height);

        for py in y..max_y {
            self.put_pixel(x, py, color);
        }
    }
}

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0x00, 0x00, 0x00);
    pub const WHITE: Self = Self::rgb(0xff, 0xff, 0xff);

    pub const RED: Self = Self::rgb(0xff, 0x00, 0x00);
    pub const GREEN: Self = Self::rgb(0x00, 0xff, 0x00);
    pub const BLUE: Self = Self::rgb(0x00, 0x00, 0xff);

    pub const YELLOW: Self = Self::rgb(0xff, 0xff, 0x00);
    pub const CYAN: Self = Self::rgb(0x00, 0xff, 0xff);
    pub const MAGENTA: Self = Self::rgb(0xff, 0x00, 0xff);

    pub const DARK_GRAY: Self = Self::rgb(0x20, 0x20, 0x20);
    pub const GRAY: Self = Self::rgb(0x80, 0x80, 0x80);
    pub const LIGHT_GRAY: Self = Self::rgb(0xc0, 0xc0, 0xc0);

    pub const ROOTLEAF_BG: Self = Self::rgb(0x10, 0x30, 0x10);
    pub const ROOTLEAF_GREEN: Self = Self::rgb(0x55, 0xff, 0x55);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            a: 0x00,
        }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}