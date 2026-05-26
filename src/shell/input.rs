use crate::shell::commands::execute_command;
use crate::shell::path::clear_line_buffer;
use crate::shell::tabs::{show_disks_tab, show_help_tab, show_mem_tab};

pub fn handle_input_byte(byte: u8, line: &mut [u8; 128], line_len: &mut usize) {
    match byte {
        crate::kernel::input::KEY_F1 => {
            *line_len = 0;
            clear_line_buffer(line);
            show_help_tab();
        }

        crate::kernel::input::KEY_F2 => {
            *line_len = 0;
            clear_line_buffer(line);
            show_mem_tab();
        }

        crate::kernel::input::KEY_F3 => {
            *line_len = 0;
            clear_line_buffer(line);
            show_disks_tab();
        }

        crate::kernel::input::KEY_ESC => {
            *line_len = 0;
            clear_line_buffer(line);
            crate::kernel::clear_console();
        }

        b'\n' | b'\r' => {
            crate::print!("\n");

            let command = &line[..*line_len];
            execute_command(command);

            *line_len = 0;
            clear_line_buffer(line);

            crate::kernel::prompt();
        }

        b'\x08' => {
            if *line_len > 0 {
                *line_len -= 1;
                line[*line_len] = 0;
                crate::print!("\x08 \x08");
            }
        }

        byte => {
            if byte < 0x20 || byte > 0x7e {
                return;
            }

            if *line_len >= line.len() - 1 {
                return;
            }

            line[*line_len] = byte;
            *line_len += 1;

            crate::kernel::write_byte(byte);
        }
    }
}
