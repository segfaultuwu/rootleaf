use core::fmt;

use crate::drivers::graphics::psf::Psf2;

#[derive(Clone, Copy)]
pub struct ConsoleColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ConsoleColor {
    pub const BLACK: Self = Self::rgb(0x00, 0x00, 0x00);
    pub const WHITE: Self = Self::rgb(0xff, 0xff, 0xff);
    pub const GREEN: Self = Self::rgb(0x00, 0xff, 0x00);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub struct ConsoleDriver {
    pub font: Psf2,
    pub fb: &'static mut [u8],
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bytes_per_pixel: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub cursor_visible: bool,
    pub blink_counter: usize,
}

impl ConsoleDriver {
    pub fn new(
        font: Psf2,
        fb: &'static mut [u8],
        width: usize,
        height: usize,
        pitch: usize,
        bytes_per_pixel: usize,
    ) -> Self {
        Self {
            font,
            fb,
            width,
            height,
            pitch,
            bytes_per_pixel,
            cursor_x: 0,
            cursor_y: 0,
            cursor_visible: false,
            blink_counter: 0,
        }
    }

    pub fn clear(&mut self, color: ConsoleColor) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, color);
            }
        }

        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn draw_cursor(&mut self, fg: ConsoleColor, bg: ConsoleColor) {
        let glyph_width = self.font_width();
        let glyph_height = self.font_height();

        let base_x = self.cursor_x * glyph_width;
        let base_y = self.cursor_y * glyph_height;

        for y in 0..glyph_height {
            for x in 0..glyph_width {
                let color = if (x + y) % 2 == 0 { fg } else { bg };
                self.put_pixel(base_x + x, base_y + y, color);
            }
        }
    }

    pub fn tick_cursor(&mut self) {
        self.blink_counter = self.blink_counter.wrapping_add(1);
        // Toggle roughly every 500000 ticks; caller decides tick frequency.
        if self.blink_counter % 500000 != 0 {
            return;
        }

        self.cursor_visible = !self.cursor_visible;

        if self.cursor_visible {
            self.draw_cursor(ConsoleColor::WHITE, ConsoleColor::BLACK);
        } else {
            // Erase cursor by redrawing a space at cursor position.
            self.draw_char(b' ', ConsoleColor::WHITE, ConsoleColor::BLACK);
        }
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, color: ConsoleColor) {
        if x >= self.width || y >= self.height {
            return;
        }

        let offset = y * self.pitch + x * self.bytes_per_pixel;

        if offset + 2 >= self.fb.len() {
            return;
        }

        self.fb[offset + 0] = color.b;
        self.fb[offset + 1] = color.g;
        self.fb[offset + 2] = color.r;

        if self.bytes_per_pixel >= 4 && offset + 3 < self.fb.len() {
            self.fb[offset + 3] = 0x00;
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            b'\r' => self.cursor_x = 0,
            b'\t' => {
                for _ in 0..4 {
                    self.write_byte(b' ');
                }
            }
            b'\x08' => {
                // Backspace: move cursor back and erase the character
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.cols().saturating_sub(1);
                }
                self.draw_char(b' ', ConsoleColor::WHITE, ConsoleColor::BLACK);
            }
            byte => {
                self.draw_char(byte, ConsoleColor::WHITE, ConsoleColor::BLACK);
                self.cursor_x += 1;

                if self.cursor_x >= self.cols() {
                    self.newline();
                }
            }
        }
    }

    pub fn write_str_raw(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    pub fn draw_char(&mut self, byte: u8, fg: ConsoleColor, bg: ConsoleColor) {
        let glyph_width = self.font_width();
        let glyph_height = self.font_height();
        let bytes_per_glyph = self.bytes_per_glyph();
        let glyph_count = self.glyph_count();

        let glyph_index = byte as usize;

        if glyph_index >= glyph_count {
            return;
        }

        let glyphs = self.font.glyphs();
        let glyphs_ptr = glyphs.as_ptr();
        let glyphs_len = glyphs.len();
        let bytes_per_row = (glyph_width + 7) / 8;

        let base_x = self.cursor_x * glyph_width;
        let base_y = self.cursor_y * glyph_height;

        let glyph_offset = glyph_index * bytes_per_glyph;
        if glyph_offset + bytes_per_glyph > glyphs_len {
            return;
        }

        for y in 0..glyph_height {
            for x in 0..glyph_width {
                let byte_index = y * bytes_per_row + x / 8;

                if byte_index >= bytes_per_glyph {
                    continue;
                }

                let bit = 0x80 >> (x % 8);
                let enabled = unsafe { *glyphs_ptr.add(glyph_offset + byte_index) } & bit != 0;

                let color = if enabled { fg } else { bg };

                self.put_pixel(base_x + x, base_y + y, color);
            }
        }
    }

    pub fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;

        if self.cursor_y >= self.rows() {
            self.scroll();
            self.cursor_y = self.rows().saturating_sub(1);
        }
    }

    pub fn scroll(&mut self) {
        let glyph_height = self.font_height();
        let scroll_bytes = glyph_height * self.pitch;

        if scroll_bytes >= self.fb.len() {
            return;
        }

        let len = self.fb.len();

        for i in 0..(len - scroll_bytes) {
            self.fb[i] = self.fb[i + scroll_bytes];
        }

        let start = len - scroll_bytes;

        for i in start..len {
            self.fb[i] = 0;
        }
    }

    pub fn cols(&self) -> usize {
        self.width / self.font_width()
    }

    pub fn rows(&self) -> usize {
        self.height / self.font_height()
    }

    fn font_width(&self) -> usize {
        self.font.width() as usize
    }

    fn font_height(&self) -> usize {
        self.font.height() as usize
    }

    fn glyph_count(&self) -> usize {
        self.font.char_count() as usize
    }

    fn bytes_per_glyph(&self) -> usize {
        self.font.char_size() as usize
    }

}
impl fmt::Write for ConsoleDriver {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str_raw(s);
        Ok(())
    }
}