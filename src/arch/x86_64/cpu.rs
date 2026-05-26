pub fn init() {}

pub fn get_cpu_vendor() -> &'static str {
    static mut VENDOR: [u8; 13] = [0; 13];

    unsafe {
        let res = core::arch::x86_64::__cpuid_count(0, 0);

        let dst = VENDOR.as_mut_ptr();

        // Copy ebx, edx, ecx dwords (each 4 bytes) into the static buffer
        core::ptr::copy_nonoverlapping(&res.ebx as *const u32 as *const u8, dst, 4);
        core::ptr::copy_nonoverlapping(&res.edx as *const u32 as *const u8, dst.add(4), 4);
        core::ptr::copy_nonoverlapping(&res.ecx as *const u32 as *const u8, dst.add(8), 4);

        // Null-terminate
        *dst.add(12) = 0;

        let slice = core::slice::from_raw_parts(VENDOR.as_ptr(), 12);

        core::str::from_utf8_unchecked(slice)
    }
}