pub mod commands;
pub mod input;
pub mod path;
pub mod tabs;

pub struct Shell {
    line: [u8; 128],
    line_len: usize,
}

impl Shell {
    pub const fn new() -> Self {
        Self {
            line: [0; 128],
            line_len: 0,
        }
    }

    pub fn handle_input_byte(&mut self, byte: u8) {
        input::handle_input_byte(byte, &mut self.line, &mut self.line_len);
    }
}
