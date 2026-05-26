#![no_std]

pub mod syscall;
pub mod io;
pub mod process;
pub mod memory;

pub use io::{read, write, open, close};
pub use process::exit;
pub use memory::{memcpy, memset, memmove, memcmp, bcmp};
pub use syscall::init;