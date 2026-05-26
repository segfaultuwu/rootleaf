#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod boot;
mod arch;
mod drivers;
mod kernel;
mod lib;
mod fs;

#[macro_use]
mod macros;

use core::panic::PanicInfo;

use crate::boot::limine::{BASE_REVISION, FRAMEBUFFER_REQUEST};
use crate::drivers::graphics::{ConsoleColor, ConsoleDriver, Psf2};
use crate::kernel::{hlt_loop, init_console};
use crate::lib::u32_to_str;

static PSF_FONT: &[u8] = include_bytes!("../assets/ter-u16n.psf");

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    drivers::serial::init();
    drivers::serial::write_str("Rootleaf: serial initialized\n");

    if !BASE_REVISION.is_supported() {
        drivers::serial::write_str("Rootleaf: unsupported Limine base revision\n");
        hlt_loop();
    }

    let font = match Psf2::new(PSF_FONT) {
        Some(font) => font,
        None => {
            drivers::serial::write_str("Rootleaf: failed to load PSF2 font\n");
            hlt_loop();
        }
    };

    drivers::serial::write_str("Rootleaf: PSF font loaded, glyphs: ");

    let mut num_buf = [0u8; 10];
    let glyph_count = u32_to_str(font.char_count() as u32, &mut num_buf);

    drivers::serial::write_str(glyph_count);
    drivers::serial::write_str("\n");

    let framebuffer_response = match FRAMEBUFFER_REQUEST.response() {
        Some(response) => response,
        None => {
            drivers::serial::write_str("Rootleaf: no framebuffer response\n");
            hlt_loop();
        }
    };

    let framebuffer = match framebuffer_response.framebuffers().first() {
        Some(framebuffer) => framebuffer,
        None => {
            drivers::serial::write_str("Rootleaf: no framebuffer found\n");
            hlt_loop();
        }
    };

    drivers::serial::write_str("Rootleaf: framebuffer found\n");

    print_framebuffer_info(framebuffer);

    let fb_size = framebuffer.pitch as usize * framebuffer.height as usize;

    let fb_slice: &'static mut [u8] = unsafe {
        core::slice::from_raw_parts_mut(
            framebuffer.address() as *mut u8,
            fb_size,
        )
    };

    let mut console = ConsoleDriver::new(
        font,
        fb_slice,
        framebuffer.width as usize,
        framebuffer.height as usize,
        framebuffer.pitch as usize,
        (framebuffer.bpp / 8) as usize,
    );

    console.clear(ConsoleColor::BLACK);
    init_console(console);

    println!("Rootleaf booted");
    println!("Framebuffer console OK");

    drivers::serial::write_str("Rootleaf: framebuffer console initialized\n");

    drivers::keyboard::init();
    arch::x86_64::init();

    println!("Keyboard driver initialized");
    println!("Interrupts enabled");
    println!("Type something:");

    // Event-driven input: wait for a byte and print it
    loop {
        if let Some(b) = crate::kernel::input::dequeue() {
            crate::print!("{}", (b as char));
            continue;
        }

        let irq_count = crate::drivers::keyboard::take_irq_count();
        if irq_count > 0 {
            continue;
        }

        // Wait step (in case we need some time)
        // Note: the keyboard interrupt should wake us if it triggers.
        crate::drivers::keyboard::poll_once();

        // Small pause to reduce busy-wait heat while polling.
        crate::kernel::tick_cursor();
        unsafe { core::arch::asm!("pause", options(nomem, nostack)); }
    }
}

fn print_framebuffer_info(framebuffer: &limine::framebuffer::Framebuffer) {
    drivers::serial::write_str("Framebuffer info:\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("width=");
    drivers::serial::write_str(u32_to_str(framebuffer.width as u32, &mut buf));
    drivers::serial::write_str("\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("height=");
    drivers::serial::write_str(u32_to_str(framebuffer.height as u32, &mut buf));
    drivers::serial::write_str("\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("pitch=");
    drivers::serial::write_str(u32_to_str(framebuffer.pitch as u32, &mut buf));
    drivers::serial::write_str("\n");

    let mut buf = [0u8; 10];

    drivers::serial::write_str("bpp=");
    drivers::serial::write_str(u32_to_str(framebuffer.bpp as u32, &mut buf));
    drivers::serial::write_str("\n");
}