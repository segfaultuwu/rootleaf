pub const PAGE_SIZE: u64 = 4096;

pub fn align_up(addr: u64, align: u64) -> u64 {
    if align == 0 {
        return addr;
    }

    let rem = addr % align;

    if rem == 0 {
        addr
    } else {
        addr + (align - rem)
    }
}

pub fn align_down(addr: u64, align: u64) -> u64 {
    if align == 0 {
        return addr;
    }

    addr - (addr % align)
}

pub fn is_aligned(addr: u64, align: u64) -> bool {
    if align == 0 {
        return true;
    }

    addr % align == 0
}