#![no_std]

pub mod syscall;
pub mod io;
pub mod process;

pub use io::{read, write};
pub use process::exit;