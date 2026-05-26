#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod arch;
mod boot;
mod drivers;
mod fs;
mod kernel;
mod lib;
mod memory;
mod shell;

#[macro_use]
mod macros;

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

use crate::boot::limine::{BASE_REVISION, FRAMEBUFFER_REQUEST};
use crate::drivers::graphics::{ConsoleDriver, Psf2};
use crate::memory::alloc_frame;
use crate::memory::addr::PAGE_SIZE;
use crate::kernel::{hlt_loop, init_console};
use crate::lib::u32_to_str;

static PSF_FONT: &[u8] = include_bytes!("../assets/ter-u16n.psf");

pub static CURRENT_PATH: &str = "0:\\";

struct ConsoleStorage(UnsafeCell<MaybeUninit<ConsoleDriver>>);

unsafe impl Sync for ConsoleStorage {}

static CONSOLE_STORAGE: ConsoleStorage = ConsoleStorage(UnsafeCell::new(MaybeUninit::uninit()));

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

    memory::init();

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

    let fb_slice: &'static mut [u8] =
        unsafe { core::slice::from_raw_parts_mut(framebuffer.address() as *mut u8, fb_size) };

    // Try to allocate a contiguous back buffer of the same size (double buffering).
    let back_buffer: Option<&'static mut [u8]> = {
        let pages = (fb_size + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize;
        let mut start_addr: u64 = 0;
        let mut ok = true;

        for i in 0..pages {
            match alloc_frame() {
                Some(f) => {
                    if i == 0 {
                        start_addr = f.addr;
                    }
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }

        if ok {
            Some(unsafe { core::slice::from_raw_parts_mut(start_addr as *mut u8, fb_size) })
        } else {
            None
        }
    };

    let console: &'static mut ConsoleDriver = unsafe {
        (*CONSOLE_STORAGE.0.get()).write(ConsoleDriver::new(
            font,
            fb_slice,
            back_buffer,
            framebuffer.width as usize,
            framebuffer.height as usize,
            framebuffer.pitch as usize,
            (framebuffer.bpp / 8) as usize,
        ))
    };

    console.init();
    init_console(console);

    drivers::serial::write_str("Rootleaf: framebuffer console initialized\n");

    arch::x86_64::init();
    drivers::keyboard::init();

    let mut shell = crate::shell::Shell::new();

    loop {
        if let Some(byte) = crate::kernel::input::dequeue() {
            shell.handle_input_byte(byte);
            continue;
        }

        let irq_count = crate::drivers::keyboard::take_irq_count();

        if irq_count > 0 {
            continue;
        }

        crate::drivers::keyboard::poll_once();
        crate::kernel::tick_cursor();

        // Present double-buffer contents if enabled
        crate::kernel::present();

        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
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
