use core::cell::UnsafeCell;
use core::fmt::{self, Write};

use crate::drivers::graphics::console::ConsoleDriver;

struct ConsoleSlot(UnsafeCell<Option<ConsoleDriver>>);

unsafe impl Sync for ConsoleSlot {}

static CONSOLE: ConsoleSlot = ConsoleSlot(UnsafeCell::new(None));

pub fn init(console: ConsoleDriver) {
    unsafe {
        *CONSOLE.0.get() = Some(console);
    }
}

pub struct KernelWriter;

impl Write for KernelWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            match (*CONSOLE.0.get()).as_mut() {
                Some(console) => {
                    console.write_str_raw(s);
                }
                None => {
                    crate::drivers::serial::write_str(s);
                }
            }
        }

        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    KernelWriter.write_fmt(args).ok();
}