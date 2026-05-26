.intel_syntax noprefix

.section .text

.global context_switch
.type context_switch, @function
context_switch:
    /*
        SysV x86_64 ABI:

        rdi = old_rsp_ptr
        rsi = new_rsp
    */

    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15

    mov qword ptr [rdi], rsp
    mov rsp, rsi

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp

    ret


.global task_entry_trampoline
.type task_entry_trampoline, @function
task_entry_trampoline:
    /*
        Initial register state after context_switch:

        r12 = entry function
        r13 = argument

        We call:

            entry(arg)
    */

    mov rdi, r13
    call r12

.task_exit_hang:
    hlt
    jmp .task_exit_hang