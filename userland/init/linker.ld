ENTRY(_start)

PHDRS
{
    text PT_LOAD FLAGS(5); /* R + X */
    data PT_LOAD FLAGS(6); /* R + W */
}

SECTIONS
{
    . = 0xffffffff80200000;

    .text : ALIGN(4K) {
        *(.text .text.*)
        *(.rodata .rodata.*)
    } :text

    .data : ALIGN(4K) {
        *(.data .data.*)
    } :data

    .bss : ALIGN(4K) {
        *(COMMON)
        *(.bss .bss.*)
    } :data
}