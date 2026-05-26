use core::cell::UnsafeCell;
use core::fmt::{self, Write};

use crate::drivers::graphics::console::ConsoleDriver;

struct ConsoleSlot(UnsafeCell<Option<&'static mut ConsoleDriver>>);

unsafe impl Sync for ConsoleSlot {}

static CONSOLE: ConsoleSlot = ConsoleSlot(UnsafeCell::new(None));

pub fn init(console: &'static mut ConsoleDriver) {
    unsafe {
        *CONSOLE.0.get() = Some(console);
    }
}

pub fn with_console<R>(f: impl FnOnce(&mut ConsoleDriver) -> R) -> Option<R> {
    unsafe {
        let slot = &mut *CONSOLE.0.get();

        match slot.as_mut() {
            Some(console) => Some(f(*console)),
            None => None,
        }
    }
}

pub fn tick_cursor() {
    let _ = with_console(|console| {
        console.tick_cursor();
    });
}

pub fn prompt() {
    let _ = with_console(|console| {
        console.prompt();
    });
}

pub fn clear_console() {
    let _ = with_console(|console| {
        console.clear_screen();
        console.prompt();
    });
}

pub fn write_raw(s: &str) {
    if with_console(|console| {
        console.write_str_raw(s);
    })
    .is_none()
    {
        crate::drivers::serial::write_str(s);
    }
}

pub struct KernelWriter;

impl Write for KernelWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_raw(s);
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    let _ = KernelWriter.write_fmt(args);
}

pub fn write_byte(byte: u8) {
    if with_console(|console| {
        console.write_byte(byte);
    })
    .is_none()
    {
        crate::drivers::serial::write_byte(byte);
    }
}
