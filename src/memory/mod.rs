pub mod addr;
pub mod frame;
pub mod info;

pub use frame::{
    alloc_frame,
    remaining_frames,
};

pub use info::{
    memory_info,
    memory_type_name,
    print_memory_map,
    MemoryInfo,
};

pub fn init() {
    if frame::init() {
        crate::print!("Memory: frame allocator initialized\n");
    } else {
        crate::print!("Memory: failed to initialize frame allocator\n");
    }
}