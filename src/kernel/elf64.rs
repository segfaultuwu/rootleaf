const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;

const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;

const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;
const EM_X86_64: u16 = 62;

const PT_LOAD: u32 = 1;
const ELF_ARENA_SIZE: usize = 16 * 1024 * 1024;

static mut ELF_ARENA: [u8; ELF_ARENA_SIZE] = [0u8; ELF_ARENA_SIZE];

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

#[derive(Clone, Copy)]
struct LoadedSeg {
    vaddr: u64,
    memsz: u64,
    host_base: *mut u8,
}

type UserEntry = extern "C" fn(usize) -> isize;

fn align_up(value: usize, align: usize) -> Option<usize> {
    if align == 0 {
        return Some(value);
    }

    let mask = align - 1;
    value.checked_add(mask).map(|v| v & !mask)
}

fn invoke_user_entry(entry: *const (), syscall_ptr: usize) -> isize {
    let entry_fn: UserEntry = unsafe { core::mem::transmute(entry) };
    entry_fn(syscall_ptr)
}

fn parse_header(data: &[u8]) -> Option<&Elf64Ehdr> {
    if data.len() < core::mem::size_of::<Elf64Ehdr>() {
        return None;
    }

    let hdr = unsafe { &*(data.as_ptr() as *const Elf64Ehdr) };

    if &hdr.e_ident[0..4] != b"\x7fELF" {
        return None;
    }

    if hdr.e_ident[EI_CLASS] != ELFCLASS64
        || hdr.e_ident[EI_DATA] != ELFDATA2LSB
        || hdr.e_ident[EI_VERSION] != EV_CURRENT
    {
        return None;
    }

    if hdr.e_machine != EM_X86_64 {
        return None;
    }

    if hdr.e_type != ET_EXEC && hdr.e_type != ET_DYN {
        return None;
    }

    if hdr.e_phentsize as usize != core::mem::size_of::<Elf64Phdr>() {
        return None;
    }

    Some(hdr)
}

pub fn run(data: &[u8]) -> Result<isize, &'static str> {
    let hdr = parse_header(data).ok_or("Invalid ELF64 header")?;

    let phoff = hdr.e_phoff as usize;
    let phnum = hdr.e_phnum as usize;
    let phsz = core::mem::size_of::<Elf64Phdr>();

    if phoff.checked_add(phnum * phsz).is_none() || phoff + phnum * phsz > data.len() {
        return Err("ELF program headers out of range");
    }

    let mut segs = [LoadedSeg {
        vaddr: 0,
        memsz: 0,
        host_base: core::ptr::null_mut(),
    }; 16];
    let mut seg_count = 0usize;
    let mut arena_used = 0usize;

    for i in 0..phnum {
        let off = phoff + i * phsz;
        let ph = unsafe { &*(data.as_ptr().add(off) as *const Elf64Phdr) };

        if ph.p_type != PT_LOAD || ph.p_memsz == 0 {
            continue;
        }

        if seg_count >= segs.len() {
            return Err("Too many PT_LOAD segments");
        }

        let file_off = ph.p_offset as usize;
        let file_sz = ph.p_filesz as usize;
        let mem_sz = ph.p_memsz as usize;

        if file_sz > mem_sz {
            return Err("Segment filesz > memsz");
        }

        if file_off.checked_add(file_sz).is_none() || file_off + file_sz > data.len() {
            return Err("Segment file range out of bounds");
        }

        let seg_align = if ph.p_align > 0 {
            (ph.p_align as usize).checked_next_power_of_two().unwrap_or(16)
        } else {
            16
        };

        let aligned = align_up(arena_used, seg_align).ok_or("ELF arena overflow")?;
        let seg_end = aligned
            .checked_add(mem_sz)
            .ok_or("ELF arena overflow")?;

        if seg_end > ELF_ARENA_SIZE {
            return Err("Out of memory while loading ELF");
        }

        let dst = unsafe { &mut ELF_ARENA[aligned..seg_end] };

        dst[..file_sz].copy_from_slice(&data[file_off..file_off + file_sz]);
        if mem_sz > file_sz {
            for b in &mut dst[file_sz..mem_sz] {
                *b = 0;
            }
        }

        segs[seg_count] = LoadedSeg {
            vaddr: ph.p_vaddr,
            memsz: ph.p_memsz,
            host_base: dst.as_mut_ptr(),
        };
        seg_count += 1;
        arena_used = seg_end;
    }

    if seg_count == 0 {
        return Err("No loadable segments");
    }

    let entry = hdr.e_entry;
    let mut host_entry: Option<*mut u8> = None;

    for seg in &segs[..seg_count] {
        if entry >= seg.vaddr && entry < seg.vaddr + seg.memsz {
            let delta = (entry - seg.vaddr) as usize;
            host_entry = Some(unsafe { seg.host_base.add(delta) });
            break;
        }
    }

    let host_entry = host_entry.ok_or("Entry not in PT_LOAD segment")?;

    crate::kernel::syscall::reset_process_state();

    let ret = invoke_user_entry(
        host_entry as *const (),
        crate::kernel::syscall::linux_syscall as *const () as usize,
    );

    if let Some(code) = crate::kernel::syscall::take_exit_code() {
        Ok(code)
    } else {
        Ok(ret)
    }
}
