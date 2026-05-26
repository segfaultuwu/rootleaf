use core::cell::UnsafeCell;

use crate::boot::limine::MEMORY_MAP_REQUEST;
use crate::memory::addr::{align_up, PAGE_SIZE};

struct FrameAllocatorSlot {
    inner: UnsafeCell<FrameAllocator>,
}

unsafe impl Sync for FrameAllocatorSlot {}

static FRAME_ALLOCATOR: FrameAllocatorSlot = FrameAllocatorSlot {
    inner: UnsafeCell::new(FrameAllocator::empty()),
};

#[derive(Clone, Copy)]
pub struct Frame {
    pub addr: u64,
}

pub struct FrameAllocator {
    current: u64,
    end: u64,
    initialized: bool,
}

impl FrameAllocator {
    pub const fn empty() -> Self {
        Self {
            current: 0,
            end: 0,
            initialized: false,
        }
    }

    pub fn init_from_memory_map(&mut self) -> bool {
        let response = match MEMORY_MAP_REQUEST.response() {
            Some(response) => response,
            None => return false,
        };

        for entry in response.entries() {
            let kind = entry.type_ as u64;

            // 0 = usable
            if kind != 0 {
                continue;
            }

            let start = align_up(entry.base, PAGE_SIZE);
            let end = entry.base.saturating_add(entry.length);

            if end <= start {
                continue;
            }

            self.current = start;
            self.end = end;
            self.initialized = true;
            return true;
        }

        false
    }

    pub fn alloc(&mut self) -> Option<Frame> {
        if !self.initialized {
            return None;
        }

        let next = self.current.saturating_add(PAGE_SIZE);

        if next > self.end {
            return None;
        }

        let frame = Frame {
            addr: self.current,
        };

        self.current = next;

        Some(frame)
    }

    pub fn remaining_frames(&self) -> u64 {
        if !self.initialized || self.current >= self.end {
            return 0;
        }

        (self.end - self.current) / PAGE_SIZE
    }
}

pub fn init() -> bool {
    unsafe {
        (*FRAME_ALLOCATOR.inner.get()).init_from_memory_map()
    }
}

pub fn alloc_frame() -> Option<Frame> {
    unsafe {
        (*FRAME_ALLOCATOR.inner.get()).alloc()
    }
}

pub fn remaining_frames() -> u64 {
    unsafe {
        (*FRAME_ALLOCATOR.inner.get()).remaining_frames()
    }
}