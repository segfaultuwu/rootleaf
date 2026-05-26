use core::panic::PanicInfo;

use crate::{CONSOLE_STORAGE, drivers::serial};

pub fn hlt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[panic_handler]
pub fn panic_handler(_info: &PanicInfo) -> ! {
    serial::write_str("Kernel panic: ");
    if let Some(location) = _info.location() {
        let mut buf = &mut [0u8; 10];
        serial::write_str(location.file());
        serial::write_str(":");
        serial::write_str(crate::u32_to_str(location.line(), buf));
        serial::write_str(":");
        serial::write_str(crate::u32_to_str(location.column(), buf));
        serial::write_str(": ");
    }
    hlt_loop();
}
