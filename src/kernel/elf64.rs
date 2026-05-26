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

fn parse_header(data: &[u8]) -> Option<&Elf64Ehdr> {
    if data.len() < core::mem::size_of::<Elf64Ehdr>() {
        return None;
    }

    let hdr = unsafe { &*(data.as_ptr() as *const Elf64Ehdr) };

    if &hdr.e_ident[0..4] != b"\x7fELF" {
        return None;
    }

    if hdr.e_ident[EI_CLASS] != ELFCLASS64 {
        return None;
    }

    if hdr.e_ident[EI_DATA] != ELFDATA2LSB {
        return None;
    }

    if hdr.e_ident[EI_VERSION] != EV_CURRENT {
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

fn phdr_at(data: &[u8], phoff: usize, index: usize) -> Option<&Elf64Phdr> {
    let phsz = core::mem::size_of::<Elf64Phdr>();
    let off = phoff.checked_add(index.checked_mul(phsz)?)?;

    if off.checked_add(phsz)? > data.len() {
        return None;
    }

    Some(unsafe { &*(data.as_ptr().add(off) as *const Elf64Phdr) })
}

fn validate_phdr_table(data: &[u8], hdr: &Elf64Ehdr) -> Result<(), &'static str> {
    let phoff = hdr.e_phoff as usize;
    let phnum = hdr.e_phnum as usize;
    let phsz = core::mem::size_of::<Elf64Phdr>();

    let table_size = phnum
        .checked_mul(phsz)
        .ok_or("ELF program header table overflow")?;

    let table_end = phoff
        .checked_add(table_size)
        .ok_or("ELF program header table overflow")?;

    if table_end > data.len() {
        return Err("ELF program headers out of range");
    }

    Ok(())
}

fn load_segment(data: &[u8], ph: &Elf64Phdr) -> Result<(), &'static str> {
    if ph.p_memsz == 0 {
        return Ok(());
    }

    if ph.p_filesz > ph.p_memsz {
        return Err("ELF segment filesz > memsz");
    }

    if ph.p_vaddr == 0 {
        return Err("ELF segment has null vaddr");
    }

    let file_off = ph.p_offset as usize;
    let file_sz = ph.p_filesz as usize;
    let mem_sz = ph.p_memsz as usize;
    let vaddr = ph.p_vaddr as usize;

    let file_end = file_off
        .checked_add(file_sz)
        .ok_or("ELF segment file range overflow")?;

    if file_end > data.len() {
        return Err("ELF segment file range out of bounds");
    }

    let mem_end = vaddr
        .checked_add(mem_sz)
        .ok_or("ELF segment memory range overflow")?;

    crate::drivers::serial::write_str("ELF: PT_LOAD vaddr=");
    crate::drivers::serial::write_hex(vaddr);
    crate::drivers::serial::write_str(" end=");
    crate::drivers::serial::write_hex(mem_end);
    crate::drivers::serial::write_str(" offset=");
    crate::drivers::serial::write_hex(file_off);
    crate::drivers::serial::write_str(" filesz=");
    crate::drivers::serial::write_hex(file_sz);
    crate::drivers::serial::write_str(" memsz=");
    crate::drivers::serial::write_hex(mem_sz);
    crate::drivers::serial::write_str(" flags=");
    crate::drivers::serial::write_hex(ph.p_flags as usize);
    crate::drivers::serial::write_str("\n");

    let src = &data[file_off..file_end];
    let dst = vaddr as *mut u8;

    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), dst, file_sz);

        if mem_sz > file_sz {
            core::ptr::write_bytes(dst.add(file_sz), 0, mem_sz - file_sz);
        }
    }

    Ok(())
}

fn entry_in_load_segment(data: &[u8], hdr: &Elf64Ehdr) -> Result<bool, &'static str> {
    let phoff = hdr.e_phoff as usize;
    let phnum = hdr.e_phnum as usize;
    let entry = hdr.e_entry;

    for i in 0..phnum {
        let ph = phdr_at(data, phoff, i).ok_or("Invalid program header")?;

        if ph.p_type != PT_LOAD {
            continue;
        }

        let start = ph.p_vaddr;
        let end = ph
            .p_vaddr
            .checked_add(ph.p_memsz)
            .ok_or("ELF segment range overflow")?;

        if entry >= start && entry < end {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn run(data: &[u8]) -> Result<isize, &'static str> {
    crate::drivers::serial::write_str("ELF: run begin\n");

    let hdr = parse_header(data).ok_or("Invalid ELF64 header")?;

    crate::drivers::serial::write_str("ELF: header parsed\n");

    if hdr.e_type == ET_DYN {
        crate::drivers::serial::write_str("ELF: warning: ET_DYN loaded without relocations\n");
    }

    validate_phdr_table(data, hdr)?;

    crate::drivers::serial::write_str("ELF: program headers range ok\n");

    if !entry_in_load_segment(data, hdr)? {
        return Err("Entry not in PT_LOAD segment");
    }

    let phoff = hdr.e_phoff as usize;
    let phnum = hdr.e_phnum as usize;

    let mut load_count = 0usize;

    for i in 0..phnum {
        let ph = phdr_at(data, phoff, i).ok_or("Invalid program header")?;

        if ph.p_type != PT_LOAD {
            continue;
        }

        load_segment(data, ph)?;
        load_count += 1;
    }

    if load_count == 0 {
        return Err("No loadable segments");
    }

    crate::drivers::serial::write_str("ELF: segments loaded\n");

    let entry = hdr.e_entry as usize;

    crate::drivers::serial::write_str("ELF: entry=");
    crate::drivers::serial::write_hex(entry);
    crate::drivers::serial::write_str("\n");

    crate::kernel::syscall::reset_process_state();

    match crate::scheduler::spawn(entry, crate::kernel::syscall::linux_syscall as usize) {
        Ok(tid) => {
            crate::kernel::syscall::set_foreground_task(tid);

            crate::drivers::serial::write_str("ELF: task spawned\n");

            let code = loop {
                if let Some(code) = crate::kernel::syscall::take_exit_code() {
                    break code;
                }

                crate::scheduler::yield_now();
            };

            crate::kernel::syscall::set_foreground_task(0);

            crate::drivers::serial::write_str("ELF: task exited\n");
            Ok(code)
        }

        Err(_) => Err("Failed to spawn task"),
    }
}
