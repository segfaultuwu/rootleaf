// src/interrupts/idt.rs

use core::arch::asm;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    options: u16,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            options: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub fn set_handler(&mut self, handler: extern "x86-interrupt" fn()) {
        let addr = handler as u64;

        self.offset_low = addr as u16;
        self.selector = 0x08; // kernel code segment
        self.options = 0x008E; // present, ring 0, interrupt gate
        self.offset_mid = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.reserved = 0;
    }

    pub fn set_handler_with_error_code(
        &mut self,
        handler: extern "x86-interrupt" fn(InterruptStackFrame, u64),
    ) {
        let addr = handler as u64;

        self.offset_low = addr as u16;
        self.selector = 0x08;
        self.options = 0x008E;
        self.offset_mid = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.reserved = 0;
    }

    pub fn set_handler_with_stack_frame(
        &mut self,
        handler: extern "x86-interrupt" fn(InterruptStackFrame),
    ) {
        let addr = handler as u64;

        self.offset_low = addr as u16;
        self.selector = 0x08;
        self.options = 0x008E;
        self.offset_mid = (addr >> 16) as u16;
        self.offset_high = (addr >> 32) as u32;
        self.reserved = 0;
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct InterruptStackFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

#[repr(C, packed)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

pub fn init_idt() {
    unsafe {
        IDT[3].set_handler_with_stack_frame(breakpoint_handler);
        IDT[8].set_handler_with_error_code(double_fault_handler);
        IDT[14].set_handler_with_error_code(page_fault_handler);

        IDT[33].set_handler_with_stack_frame(keyboard_interrupt_handler);

        let idt_ptr = IdtPointer {
            limit: core::mem::size_of::<[IdtEntry; 256]>() as u16 - 1,
            base: core::ptr::addr_of!(IDT) as u64,
        };

        core::arch::asm!("lidt [{}]", in(reg) &idt_ptr, options(readonly, nostack));
    }
}
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::println!("EXCEPTION: BREAKPOINT");
    crate::println!("{:#x?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    crate::println!("EXCEPTION: DOUBLE FAULT");
    crate::println!("error code: {}", error_code);
    crate::println!("{:#x?}", stack_frame);

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    let cr2: u64;

    unsafe {
        asm!("mov {}, cr2", out(reg) cr2);
    }

    crate::println!("EXCEPTION: PAGE FAULT");
    crate::println!("accessed address: {:#x}", cr2);
    crate::println!("error code: {:#x}", error_code);
    crate::println!("{:#x?}", stack_frame);

    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::drivers::serial::write_str("[int] keyboard handler enter\n");
    crate::drivers::keyboard::handle_interrupt();
    crate::drivers::serial::write_str("[int] keyboard handler after handle_interrupt\n");

    unsafe {
        crate::drivers::serial::write_str("[int] keyboard handler sending EOI\n");
        crate::arch::x86_64::pic::send_eoi(1);
        crate::drivers::serial::write_str("[int] keyboard handler sent EOI\n");
    }
}