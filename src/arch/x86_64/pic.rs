use crate::arch::x86_64::port::{inb, io_wait, outb};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = 40;

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;

const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

pub unsafe fn remap() {
    let pic1_mask = unsafe { inb(PIC1_DATA) };
    let pic2_mask = unsafe { inb(PIC2_DATA) };

    // ICW1: start initialization
    unsafe {
        outb(PIC1_COMMAND, 0x11);
        io_wait();
        outb(PIC2_COMMAND, 0x11);
        io_wait();

        // ICW2: vector offsets
        outb(PIC1_DATA, PIC_1_OFFSET);
        io_wait();
        outb(PIC2_DATA, PIC_2_OFFSET);
        io_wait();

        // ICW3: tell master/slave relationship
        outb(PIC1_DATA, 4);
        io_wait();
        outb(PIC2_DATA, 2);
        io_wait();

        // ICW4: 8086 mode
        outb(PIC1_DATA, 0x01);
        io_wait();
        outb(PIC2_DATA, 0x01);
        io_wait();

        // Restore saved masks
        outb(PIC1_DATA, pic1_mask);
        outb(PIC2_DATA, pic2_mask);
    }
}

pub unsafe fn enable_irq(irq: u8) {
    let port = if irq < 8 {
        0x21
    } else {
        0xA1
    };

    let irq_line = if irq < 8 {
        irq
    } else {
        irq - 8
    };

    unsafe {
        let mask = crate::arch::x86_64::port::inb(port);
        crate::arch::x86_64::port::outb(port, mask & !(1 << irq_line));
    }
}

pub unsafe fn disable_irq(irq: u8) {
    let port = if irq < 8 {
        PIC1_DATA
    } else {
        PIC2_DATA
    };

    let irq_line = if irq < 8 {
        irq
    } else {
        irq - 8
    };

    unsafe {
        let mask = inb(port) | (1 << irq_line);
        outb(port, mask);
    }
}

pub unsafe fn send_eoi(irq: u8) {
    unsafe {
        if irq >= 8 {
            outb(PIC2_COMMAND, PIC_EOI);
        }

        outb(PIC1_COMMAND, PIC_EOI);
    }
}