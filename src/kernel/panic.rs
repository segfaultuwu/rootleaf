use core::panic::PanicInfo;

pub fn hlt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[panic_handler]
pub fn panic_handler(_info: &PanicInfo) -> ! {
    hlt_loop();
}
