use core::sync::atomic::{AtomicBool, Ordering};

static mut CPU_VENDOR: [u8; 13] = [0; 13];
static CPU_VENDOR_READY: AtomicBool = AtomicBool::new(false);

pub fn init() {
    detect_cpu_vendor();
}

fn detect_cpu_vendor() {
    let vendor = cpuid_vendor();

    unsafe {
        let ptr = core::ptr::addr_of_mut!(CPU_VENDOR) as *mut u8;

        for i in 0..13 {
            core::ptr::write(ptr.add(i), vendor[i]);
        }
    }

    CPU_VENDOR_READY.store(true, Ordering::SeqCst);
}

fn cpuid_vendor() -> [u8; 13] {
    let mut vendor = [0u8; 13];

    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::__cpuid;

    let result = __cpuid(0);

    let ebx = result.ebx.to_le_bytes();
    let edx = result.edx.to_le_bytes();
    let ecx = result.ecx.to_le_bytes();

    vendor[0] = ebx[0];
    vendor[1] = ebx[1];
    vendor[2] = ebx[2];
    vendor[3] = ebx[3];

    vendor[4] = edx[0];
    vendor[5] = edx[1];
    vendor[6] = edx[2];
    vendor[7] = edx[3];

    vendor[8] = ecx[0];
    vendor[9] = ecx[1];
    vendor[10] = ecx[2];
    vendor[11] = ecx[3];

    vendor[12] = 0;

    vendor
}

pub fn get_cpu_vendor() -> &'static str {
    if !CPU_VENDOR_READY.load(Ordering::SeqCst) {
        return "unknown";
    }

    unsafe {
        let ptr = core::ptr::addr_of!(CPU_VENDOR) as *const u8;

        let slice = core::slice::from_raw_parts(ptr, 12);

        core::str::from_utf8_unchecked(slice)
    }
}
