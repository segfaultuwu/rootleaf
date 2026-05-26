use core::fmt;

use crate::drivers::graphics::psf::Psf2;

const TOP_BAR_ROWS: usize = 2;
const BOTTOM_BAR_ROWS: usize = 1;

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

    pub const DARK_BLUE: Self = Self::rgb(0x00, 0x00, 0xaa);
    pub const BLUE: Self = Self::rgb(0x00, 0x00, 0xff);
    pub const CYAN: Self = Self::rgb(0x00, 0xaa, 0xaa);
    pub const GRAY: Self = Self::rgb(0xaa, 0xaa, 0xaa);
    pub const DARK_GRAY: Self = Self::rgb(0x20, 0x20, 0x20);
    pub const YELLOW: Self = Self::rgb(0xff, 0xff, 0x00);
    pub const RED: Self = Self::rgb(0xff, 0x00, 0x00);
    pub const LIGHT_GREEN: Self = Self::rgb(0x55, 0xff, 0x55);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub struct ConsoleDriver {
    pub font: Psf2,
    pub fb: &'static mut [u8],
    pub back_buffer: Option<&'static mut [u8]>,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
    pub bytes_per_pixel: usize,
    pub dirty: bool,

    pub cursor_x: usize,
    pub cursor_y: usize,

    pub cursor_visible: bool,
    pub blink_counter: usize,

    pub fg: ConsoleColor,
    pub bg: ConsoleColor,
}

impl ConsoleDriver {
    pub fn new(
        font: Psf2,
        fb: &'static mut [u8],
        back_buffer: Option<&'static mut [u8]>,
        width: usize,
        height: usize,
        pitch: usize,
        bytes_per_pixel: usize,
    ) -> Self {
        Self {
            font,
            fb,
            back_buffer,
            width,
            height,
            pitch,
            bytes_per_pixel,

            cursor_x: 0,
            cursor_y: TOP_BAR_ROWS,

            cursor_visible: false,
            blink_counter: 0,

            fg: ConsoleColor::WHITE,
            bg: ConsoleColor::BLACK,
            dirty: false,
        }
    }

    fn framebuffer_mut(&mut self) -> &mut [u8] {
        match self.back_buffer {
            Some(ref mut b) => b,
            None => self.fb,
        }
    }

    pub fn present(&mut self) {
        if !self.dirty {
            return;
        }

        if let Some(ref b) = self.back_buffer {
            let src: &[u8] = &*b;
            let dst: &mut [u8] = &mut *self.fb;

            let len = core::cmp::min(dst.len(), src.len());

            let src_start = src.as_ptr() as usize;
            let src_end = src_start.saturating_add(len);
            let dst_start = dst.as_ptr() as usize;
            let dst_end = dst_start.saturating_add(len);

            let overlaps = src_start < dst_end && dst_start < src_end;

            if overlaps {
                if dst_start <= src_start {
                    for index in 0..len {
                        dst[index] = src[index];
                    }
                } else {
                    let mut index = len;
                    while index > 0 {
                        index -= 1;
                        dst[index] = src[index];
                    }
                }
            } else {
                dst[..len].copy_from_slice(&src[..len]);
            }
            // clear dirty flag after presenting
            self.dirty = false;
        }
    }

    pub fn init(&mut self) {
        self.clear_screen();
        self.draw_banner();
        self.prompt();
    }

    pub fn clear_screen(&mut self) {
        self.clear(ConsoleColor::BLACK);
        self.cursor_x = 0;
        self.cursor_y = self.content_start_row();
        self.redraw_chrome();
    }

    pub fn redraw_chrome(&mut self) {
        self.draw_top_bar();
        self.draw_bottom_bar();
    }

    pub fn clear(&mut self, color: ConsoleColor) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, color);
            }
        }

        self.cursor_x = 0;
        self.cursor_y = self.content_start_row();
    }

    pub fn set_colors(&mut self, fg: ConsoleColor, bg: ConsoleColor) {
        self.fg = fg;
        self.bg = bg;
    }

    pub fn reset_colors(&mut self) {
        self.fg = ConsoleColor::WHITE;
        self.bg = ConsoleColor::BLACK;
    }

    pub fn draw_top_bar(&mut self) {
        let old_x = self.cursor_x;
        let old_y = self.cursor_y;

        self.fill_text_row(0, ConsoleColor::DARK_BLUE);
        self.fill_text_row(1, ConsoleColor::BLACK);

        let cols = self.cols();

        self.write_str_at(
            " ROOTLEAF KERNEL",
            0,
            0,
            ConsoleColor::YELLOW,
            ConsoleColor::DARK_BLUE,
        );

        let mem = crate::memory::info::memory_info();

        let mut mem_buf = [0u8; 20];
        let mem_str = crate::lib::u64_to_str(mem.usable_mib(), &mut mem_buf);

        let prefix = "MEM: ";
        let suffix = " MiB | Framebuffer | PS/2";

        let right_len = prefix.len() + mem_str.len() + suffix.len();
        let right_col = cols.saturating_sub(right_len + 1);

        self.write_str_at(
            prefix,
            right_col,
            0,
            ConsoleColor::WHITE,
            ConsoleColor::DARK_BLUE,
        );

        self.write_str_at(
            mem_str,
            right_col + prefix.len(),
            0,
            ConsoleColor::YELLOW,
            ConsoleColor::DARK_BLUE,
        );

        self.write_str_at(
            suffix,
            right_col + prefix.len() + mem_str.len(),
            0,
            ConsoleColor::WHITE,
            ConsoleColor::DARK_BLUE,
        );

        for col in 0..cols {
            self.write_byte_at(b'-', col, 1, ConsoleColor::DARK_GRAY, ConsoleColor::BLACK);
        }

        self.cursor_x = old_x;
        self.cursor_y = old_y;
    }

    pub fn draw_bottom_bar(&mut self) {
        let old_x = self.cursor_x;
        let old_y = self.cursor_y;

        let row = self.bottom_bar_row();

        self.fill_text_row(row, ConsoleColor::DARK_BLUE);

        self.write_str_at(
            " F1 Help | F2 Mem | F3 Disks | ESC Shell",
            0,
            row,
            ConsoleColor::WHITE,
            ConsoleColor::DARK_BLUE,
        );

        let right = env!("CARGO_PKG_VERSION");
        let col = self.cols().saturating_sub(right.len() + 11);

        self.write_str_at(
            "Rootleaf ",
            col,
            row,
            ConsoleColor::GRAY,
            ConsoleColor::DARK_BLUE,
        );

        self.write_str_at(
            right,
            col + 9,
            row,
            ConsoleColor::YELLOW,
            ConsoleColor::DARK_BLUE,
        );

        self.cursor_x = old_x;
        self.cursor_y = old_y;
    }

    pub fn draw_banner(&mut self) {
        self.write_colored(
            "Rootleaf Kernel [Version ",
            ConsoleColor::GREEN,
            ConsoleColor::BLACK,
        );

        self.write_colored(
            env!("CARGO_PKG_VERSION"),
            ConsoleColor::CYAN,
            ConsoleColor::BLACK,
        );

        self.write_colored("]\n", ConsoleColor::GREEN, ConsoleColor::BLACK);

        self.write_colored(
            "(C) 2026 Rootleaf community\n",
            ConsoleColor::GRAY,
            ConsoleColor::BLACK,
        );

        self.write_colored("\n", ConsoleColor::WHITE, ConsoleColor::BLACK);

        self.write_colored("Type ", ConsoleColor::GRAY, ConsoleColor::BLACK);

        self.write_colored("HELP", ConsoleColor::CYAN, ConsoleColor::BLACK);

        self.write_colored(
            " for available commands.\n\n",
            ConsoleColor::GRAY,
            ConsoleColor::BLACK,
        );
    }

    pub fn prompt(&mut self) {
        self.write_colored(crate::fs::cwd::get(), ConsoleColor::LIGHT_GREEN, ConsoleColor::BLACK);

        self.write_colored(" % ", ConsoleColor::LIGHT_GREEN, ConsoleColor::BLACK);

        self.write_byte(b' ');
    }

    pub fn draw_cursor(&mut self, fg: ConsoleColor, bg: ConsoleColor) {
        if self.cursor_y < self.content_start_row() || self.cursor_y >= self.content_end_row() {
            return;
        }

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

        if self.blink_counter % 500000 != 0 {
            return;
        }

        if self.cursor_y < self.content_start_row() || self.cursor_y >= self.content_end_row() {
            return;
        }

        self.cursor_visible = !self.cursor_visible;

        if self.cursor_visible {
            self.draw_cursor(ConsoleColor::WHITE, ConsoleColor::BLACK);
        } else {
            self.draw_char(b' ', self.fg, self.bg);
        }
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, color: ConsoleColor) {
        if x >= self.width || y >= self.height {
            return;
        }

        let offset = y * self.pitch + x * self.bytes_per_pixel;

        let bpp = self.bytes_per_pixel;
        let fb = self.framebuffer_mut();

        if offset + 2 >= fb.len() {
            return;
        }

        fb[offset] = color.b;
        fb[offset + 1] = color.g;
        fb[offset + 2] = color.r;

        if bpp >= 4 && offset + 3 < fb.len() {
            fb[offset + 3] = 0x00;
        }

        if self.back_buffer.is_some() {
            self.dirty = true;
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
            b'\x08' => self.backspace(),
            byte => {
                self.draw_char(byte, self.fg, self.bg);
                self.cursor_x += 1;

                if self.cursor_x >= self.cols() {
                    self.newline();
                }
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        } else if self.cursor_y > self.content_start_row() {
            self.cursor_y -= 1;
            self.cursor_x = self.cols().saturating_sub(1);
        } else {
            return;
        }

        self.draw_char(b' ', self.fg, self.bg);
    }

    pub fn write_colored(&mut self, text: &str, fg: ConsoleColor, bg: ConsoleColor) {
        for byte in text.bytes() {
            match byte {
                b'\n' => self.newline(),
                b'\r' => self.cursor_x = 0,
                b'\t' => {
                    for _ in 0..4 {
                        self.write_colored(" ", fg, bg);
                    }
                }
                b'\x08' => self.backspace(),
                byte => {
                    self.draw_char(byte, fg, bg);
                    self.cursor_x += 1;

                    if self.cursor_x >= self.cols() {
                        self.newline();
                    }
                }
            }
        }
    }

    pub fn write_str_raw(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    pub fn write_str_at(
        &mut self,
        text: &str,
        col: usize,
        row: usize,
        fg: ConsoleColor,
        bg: ConsoleColor,
    ) {
        let old_x = self.cursor_x;
        let old_y = self.cursor_y;

        self.cursor_x = col;
        self.cursor_y = row;

        for byte in text.bytes() {
            if self.cursor_x >= self.cols() || self.cursor_y >= self.rows() {
                break;
            }

            self.draw_char(byte, fg, bg);
            self.cursor_x += 1;
        }

        self.cursor_x = old_x;
        self.cursor_y = old_y;
    }

    pub fn write_byte_at(
        &mut self,
        byte: u8,
        col: usize,
        row: usize,
        fg: ConsoleColor,
        bg: ConsoleColor,
    ) {
        let old_x = self.cursor_x;
        let old_y = self.cursor_y;

        self.cursor_x = col;
        self.cursor_y = row;
        self.draw_char(byte, fg, bg);

        self.cursor_x = old_x;
        self.cursor_y = old_y;
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

        if base_x >= self.width || base_y >= self.height {
            return;
        }

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

        if self.cursor_y >= self.content_end_row() {
            self.scroll();
            self.cursor_y = self.content_end_row().saturating_sub(1);
        }
    }

    pub fn scroll(&mut self) {
        let glyph_height = self.font_height();

        let start_row = self.content_start_row();
        let end_row = self.content_end_row();

        if end_row <= start_row + 1 {
            return;
        }

        let start_y = start_row * glyph_height;
        let end_y = end_row * glyph_height;

        if start_y >= self.height || end_y > self.height {
            return;
        }

        let row_bytes = glyph_height * self.pitch;
        let start_offset = start_y * self.pitch;
        let end_offset = end_y * self.pitch;

        let fb = self.framebuffer_mut();

        if start_offset + row_bytes >= end_offset || end_offset > fb.len() {
            return;
        }

        let move_len = end_offset.saturating_sub(start_offset).saturating_sub(row_bytes);

        if move_len > 0 {
            fb.copy_within(start_offset + row_bytes..end_offset, start_offset);
        }

        let clear_start = end_offset - row_bytes;
        fb[clear_start..end_offset].fill(0);

        if self.back_buffer.is_some() {
            self.dirty = true;
        }

        self.redraw_chrome();
    }

    pub fn fill_text_row(&mut self, row: usize, color: ConsoleColor) {
        let glyph_height = self.font_height();
        let y_start = row * glyph_height;
        let y_end = y_start + glyph_height;

        if y_start >= self.height {
            return;
        }

        for y in y_start..y_end.min(self.height) {
            for x in 0..self.width {
                self.put_pixel(x, y, color);
            }
        }
    }

    pub fn cols(&self) -> usize {
        let w = self.font_width();

        if w == 0 {
            return 0;
        }

        self.width / w
    }

    pub fn rows(&self) -> usize {
        let h = self.font_height();

        if h == 0 {
            return 0;
        }

        self.height / h
    }

    pub fn content_start_row(&self) -> usize {
        TOP_BAR_ROWS
    }

    pub fn content_end_row(&self) -> usize {
        self.rows().saturating_sub(BOTTOM_BAR_ROWS)
    }

    pub fn bottom_bar_row(&self) -> usize {
        self.rows().saturating_sub(1)
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
