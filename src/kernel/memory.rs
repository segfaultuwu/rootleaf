use crate::boot::limine::MEMORY_MAP_REQUEST;

#[derive(Clone, Copy)]
pub struct MemoryInfo {
    pub total_ram_bytes: u64,
    pub usable_bytes: u64,
    pub reserved_bytes: u64,
    pub bootloader_reclaimable_bytes: u64,
    pub kernel_and_modules_bytes: u64,
    pub framebuffer_bytes: u64,
    pub acpi_reclaimable_bytes: u64,
    pub acpi_nvs_bytes: u64,
    pub bad_memory_bytes: u64,
    pub entry_count: usize,
}

impl MemoryInfo {
    pub const fn empty() -> Self {
        Self {
            total_ram_bytes: 0,
            usable_bytes: 0,
            reserved_bytes: 0,
            bootloader_reclaimable_bytes: 0,
            kernel_and_modules_bytes: 0,
            framebuffer_bytes: 0,
            acpi_reclaimable_bytes: 0,
            acpi_nvs_bytes: 0,
            bad_memory_bytes: 0,
            entry_count: 0,
        }
    }

    pub fn total_mib(&self) -> u64 {
        self.total_ram_bytes / 1024 / 1024
    }

    pub fn usable_mib(&self) -> u64 {
        self.usable_bytes / 1024 / 1024
    }

    pub fn reserved_mib(&self) -> u64 {
        self.reserved_bytes / 1024 / 1024
    }

    pub fn bootloader_reclaimable_mib(&self) -> u64 {
        self.bootloader_reclaimable_bytes / 1024 / 1024
    }

    pub fn kernel_and_modules_mib(&self) -> u64 {
        self.kernel_and_modules_bytes / 1024 / 1024
    }

    pub fn framebuffer_kib(&self) -> u64 {
        self.framebuffer_bytes / 1024
    }
}

const MEMMAP_USABLE: u64 = 0;
const MEMMAP_RESERVED: u64 = 1;
const MEMMAP_ACPI_RECLAIMABLE: u64 = 2;
const MEMMAP_ACPI_NVS: u64 = 3;
const MEMMAP_BAD_MEMORY: u64 = 4;
const MEMMAP_BOOTLOADER_RECLAIMABLE: u64 = 5;
const MEMMAP_KERNEL_AND_MODULES: u64 = 6;
const MEMMAP_FRAMEBUFFER: u64 = 7;

pub fn memory_info() -> MemoryInfo {
    let response = match MEMORY_MAP_REQUEST.response() {
        Some(response) => response,
        None => return MemoryInfo::empty(),
    };

    let mut info = MemoryInfo::empty();

    for entry in response.entries() {
        let length = entry.length;
        let kind = entry.type_ as u64;

        info.entry_count += 1;

        match kind {
            MEMMAP_USABLE => {
                info.usable_bytes = info.usable_bytes.saturating_add(length);
                info.total_ram_bytes = info.total_ram_bytes.saturating_add(length);
            }

            MEMMAP_BOOTLOADER_RECLAIMABLE => {
                info.bootloader_reclaimable_bytes =
                    info.bootloader_reclaimable_bytes.saturating_add(length);

                info.usable_bytes = info.usable_bytes.saturating_add(length);
                info.total_ram_bytes = info.total_ram_bytes.saturating_add(length);
            }

            MEMMAP_KERNEL_AND_MODULES => {
                info.kernel_and_modules_bytes =
                    info.kernel_and_modules_bytes.saturating_add(length);

                info.reserved_bytes = info.reserved_bytes.saturating_add(length);
                info.total_ram_bytes = info.total_ram_bytes.saturating_add(length);
            }

            MEMMAP_FRAMEBUFFER => {
                info.framebuffer_bytes = info.framebuffer_bytes.saturating_add(length);
                info.reserved_bytes = info.reserved_bytes.saturating_add(length);
                info.total_ram_bytes = info.total_ram_bytes.saturating_add(length);
            }

            MEMMAP_ACPI_RECLAIMABLE => {
                info.acpi_reclaimable_bytes = info.acpi_reclaimable_bytes.saturating_add(length);

                info.reserved_bytes = info.reserved_bytes.saturating_add(length);
                info.total_ram_bytes = info.total_ram_bytes.saturating_add(length);
            }

            MEMMAP_ACPI_NVS => {
                info.acpi_nvs_bytes = info.acpi_nvs_bytes.saturating_add(length);
                info.reserved_bytes = info.reserved_bytes.saturating_add(length);
                info.total_ram_bytes = info.total_ram_bytes.saturating_add(length);
            }

            MEMMAP_BAD_MEMORY => {
                info.bad_memory_bytes = info.bad_memory_bytes.saturating_add(length);
                info.reserved_bytes = info.reserved_bytes.saturating_add(length);
                info.total_ram_bytes = info.total_ram_bytes.saturating_add(length);
            }

            MEMMAP_RESERVED | _ => {}
        }
    }

    info
}

pub fn memory_type_name(type_: u64) -> &'static str {
    match type_ {
        MEMMAP_USABLE => "Usable",
        MEMMAP_RESERVED => "Reserved/MMIO",
        MEMMAP_ACPI_RECLAIMABLE => "ACPI reclaimable",
        MEMMAP_ACPI_NVS => "ACPI NVS",
        MEMMAP_BAD_MEMORY => "Bad memory",
        MEMMAP_BOOTLOADER_RECLAIMABLE => "Bootloader reclaimable",
        MEMMAP_KERNEL_AND_MODULES => "Kernel/modules",
        MEMMAP_FRAMEBUFFER => "Framebuffer",
        _ => "Unknown",
    }
}

pub fn print_memory_map() {
    let response = match MEMORY_MAP_REQUEST.response() {
        Some(response) => response,
        None => {
            crate::print!("No memory map available\n");
            return;
        }
    };

    crate::print!("Memory map:\n");
    crate::print!("  Base       Length     Type\n");

    for entry in response.entries() {
        let mut buf = [0u8; 20];

        crate::kernel::write_raw("  ");

        crate::kernel::write_raw(crate::lib::u64_to_hex(entry.base, &mut buf));
        crate::kernel::write_raw("  ");

        let mut buf = [0u8; 20];

        crate::kernel::write_raw(crate::lib::u64_to_str(entry.length / 1024, &mut buf));
        crate::kernel::write_raw(" KiB  ");

        crate::kernel::write_raw(memory_type_name(entry.type_ as u64));
        crate::print!("\n");
    }
}
