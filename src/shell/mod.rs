pub mod autorun;
pub mod commands;
pub mod editor;
pub mod input;
pub mod path;
pub mod tabs;

use core::cell::UnsafeCell;

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

    pub fn tick(&mut self) {
        while let Some(byte) = crate::kernel::input::dequeue() {
            self.handle_input_byte(byte);
        }

        let irq_count = crate::drivers::keyboard::take_irq_count();

        if irq_count == 0 {
            crate::drivers::keyboard::poll_once();
        }

        crate::kernel::tick_cursor();
        crate::kernel::present();
    }
}

struct ShellCell(UnsafeCell<Shell>);

unsafe impl Sync for ShellCell {}

static SHELL: ShellCell = ShellCell(UnsafeCell::new(Shell::new()));

pub extern "C" fn shell_task(_arg: usize) -> ! {
    crate::drivers::serial::write_str("[shell] shell task started\n");
    crate::shell::autorun::init();
    loop {
        unsafe {
            let shell = &mut *SHELL.0.get();
            shell.tick();
        }

        crate::scheduler::yield_now();

        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}
