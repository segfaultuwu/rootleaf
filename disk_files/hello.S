.intel_syntax noprefix
.global _start
.section .text
_start:
    mov r11, rdi
    sub rsp, 8

    mov rdi, 1
    mov rsi, 1
    lea rdx, [rip + msg]
    lea rcx, [rip + msg_end]
    sub rcx, rdx
    xor r8, r8
    xor r9, r9
    call r11

    mov rdi, 60
    xor rsi, rsi
    xor rdx, rdx
    xor rcx, rcx
    xor r8, r8
    xor r9, r9
    call r11

    add rsp, 8
    ret

.section .rodata
msg:
    .ascii "Hello, World!\n"
msg_end:
