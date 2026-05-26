.intel_syntax noprefix
.global _start
.section .text
_start:
    mov r12, rdi

    mov rdi, 1
    mov rsi, 1
    lea rdx, [rip + msg]
    mov rcx, msg_len
    xor r8, r8
    xor r9, r9
    call r12

    mov rdi, 60
    xor rsi, rsi
    xor rdx, rdx
    xor rcx, rcx
    xor r8, r8
    xor r9, r9
    call r12

    ret

.section .rodata
msg:
    .ascii "Hello, World!\n"
    .set msg_len, . - msg
