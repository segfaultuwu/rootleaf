use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::memory::frame::alloc_frame;

pub const MAX_TASKS: usize = 16;
pub const STACK_PAGES: usize = 1;
pub const PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Empty,
    Runnable,
    Running,
    Dead,
}

#[derive(Clone, Copy)]
pub struct Task {
    pub id: usize,
    pub state: TaskState,
    pub rsp: usize,
    pub stack_base: usize,
    pub stack_top: usize,
}

const EMPTY_TASK: Task = Task {
    id: 0,
    state: TaskState::Empty,
    rsp: 0,
    stack_base: 0,
    stack_top: 0,
};

struct TaskTable(UnsafeCell<[Task; MAX_TASKS]>);

unsafe impl Sync for TaskTable {}

static TASKS: TaskTable = TaskTable(UnsafeCell::new([EMPTY_TASK; MAX_TASKS]));

static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);
static CURRENT_TASK: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" {
    fn context_switch(old_rsp_ptr: *mut usize, new_rsp: usize);
    fn task_entry_trampoline() -> !;
}

pub fn init() {
    unsafe {
        let tasks = &mut *TASKS.0.get();

        tasks[0] = Task {
            id: 0,
            state: TaskState::Running,
            rsp: 0,
            stack_base: 0,
            stack_top: 0,
        };
    }

    CURRENT_TASK.store(0, Ordering::SeqCst);

    crate::drivers::serial::write_str("[sched] initialized\n");
}

#[repr(align(16))]
struct AlignedStack([u8; PAGE_SIZE]);

static mut TASK_STACKS: [AlignedStack; MAX_TASKS] =
    [const { AlignedStack([0; PAGE_SIZE]) }; MAX_TASKS];

static mut TEST_TASK_STACK: AlignedStack = AlignedStack([0; PAGE_SIZE]);

fn alloc_stack_for_slot(slot: usize) -> Option<(usize, usize)> {
    if slot >= MAX_TASKS {
        return None;
    }

    unsafe {
        crate::drivers::serial::write_str("[sched] using static stack for slot ");
        crate::drivers::serial::write_hex(slot);
        crate::drivers::serial::write_str("\n");

        let stack_base = core::ptr::addr_of_mut!(TASK_STACKS[slot].0) as usize;
        let mut stack_top = stack_base + PAGE_SIZE;

        stack_top &= !0xF;

        crate::drivers::serial::write_str("[sched] static stack ");
        crate::drivers::serial::write_hex(stack_base);
        crate::drivers::serial::write_str("..");
        crate::drivers::serial::write_hex(stack_top);
        crate::drivers::serial::write_str("\n");

        Some((stack_base, stack_top))
    }
}

pub fn spawn(entry: usize, arg: usize) -> Result<usize, &'static str> {
    unsafe {
        crate::drivers::serial::write_str("[sched] spawning task with entry ");
        crate::drivers::serial::write_hex(entry);
        crate::drivers::serial::write_str(" and arg ");
        crate::drivers::serial::write_hex(arg);
        crate::drivers::serial::write_str("\n");

        if entry == 0 {
            return Err("task entry is null");
        }

        let tasks = &mut *TASKS.0.get();

        let idx = (1..MAX_TASKS)
            .find(|&i| tasks[i].state == TaskState::Empty || tasks[i].state == TaskState::Dead)
            .ok_or("no free task slots")?;

        crate::drivers::serial::write_str("[sched] allocating stack\n");

        let (stack_base, stack_top) =
            alloc_stack_for_slot(idx).ok_or("failed to allocate task stack")?;

        crate::drivers::serial::write_str("[sched] stack allocated ");
        crate::drivers::serial::write_hex(stack_base);
        crate::drivers::serial::write_str("..");
        crate::drivers::serial::write_hex(stack_top);
        crate::drivers::serial::write_str("\n");

        let mut sp = stack_top;

        /*
            Initial stack layout.

            context_switch does:

                push rbp
                push rbx
                push r12
                push r13
                push r14
                push r15

                mov [old_rsp_ptr], rsp
                mov rsp, new_rsp

                pop r15
                pop r14
                pop r13
                pop r12
                pop rbx
                pop rbp

                ret

            So the new task stack must look like this:

                new_rsp -> r15
                           r14
                           r13 = arg
                           r12 = entry
                           rbx
                           rbp
                           return address = task_entry_trampoline
        */

        push(&mut sp, task_entry_trampoline as usize); // ret target
        push(&mut sp, 0); // rbp
        push(&mut sp, 0); // rbx
        push(&mut sp, entry); // r12 = entry address
        push(&mut sp, arg); // r13 = arg
        push(&mut sp, 0); // r14
        push(&mut sp, 0); // r15

        crate::drivers::serial::write_str("[sched] final sp=");
        crate::drivers::serial::write_hex(sp);
        crate::drivers::serial::write_str(" peek words:");

        // Print first 6 words from the new stack for verification
        for i in 0..6usize {
            let addr = (sp + i * core::mem::size_of::<usize>()) as *const usize;
            let val = unsafe { core::ptr::read(addr) };
            crate::drivers::serial::write_str(" ");
            crate::drivers::serial::write_hex(val);
        }

        crate::drivers::serial::write_str("\n");

        let id = NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst);

        tasks[idx] = Task {
            id,
            state: TaskState::Runnable,
            rsp: sp,
            stack_base,
            stack_top,
        };

        crate::drivers::serial::write_str("[sched] task spawned with id ");
        crate::drivers::serial::write_hex(id);
        crate::drivers::serial::write_str(" in slot ");
        crate::drivers::serial::write_hex(idx);
        crate::drivers::serial::write_str("\n");

        Ok(id)
    }
}

unsafe fn push(sp: &mut usize, value: usize) {
    *sp -= core::mem::size_of::<usize>();
    *(*sp as *mut usize) = value;
}

pub fn yield_now() {
    schedule();
}

pub fn schedule() {
    unsafe {
        let tasks = &mut *TASKS.0.get();
        let current = CURRENT_TASK.load(Ordering::SeqCst);

        let Some(next_idx) = find_next_runnable(tasks, current) else {
            return;
        };

        if next_idx == current {
            return;
        }

        let old_rsp_ptr = &mut tasks[current].rsp as *mut usize;
        let new_rsp = tasks[next_idx].rsp;

        if tasks[current].state == TaskState::Running {
            tasks[current].state = TaskState::Runnable;
        }

        tasks[next_idx].state = TaskState::Running;
        CURRENT_TASK.store(next_idx, Ordering::SeqCst);

        context_switch(old_rsp_ptr, new_rsp);
    }
}

fn find_next_runnable(tasks: &[Task; MAX_TASKS], current: usize) -> Option<usize> {
    for offset in 1..=MAX_TASKS {
        let idx = (current + offset) % MAX_TASKS;

        if tasks[idx].state == TaskState::Runnable {
            return Some(idx);
        }
    }

    None
}

pub fn exit_current_task() -> ! {
    unsafe {
        let tasks = &mut *TASKS.0.get();
        let current = CURRENT_TASK.load(Ordering::SeqCst);

        crate::drivers::serial::write_str("[sched] task exiting ");
        crate::drivers::serial::write_hex(current);
        crate::drivers::serial::write_str("\n");

        tasks[current].state = TaskState::Dead;
    }

    schedule();

    panic!("dead task returned to execution");
}

pub fn current_task_id() -> usize {
    let idx = CURRENT_TASK.load(Ordering::SeqCst);

    unsafe { (*TASKS.0.get())[idx].id }
}

pub fn current_task_index() -> usize {
    CURRENT_TASK.load(Ordering::SeqCst)
}

pub fn kill_task_by_id(id: usize) -> bool {
    unsafe {
        let tasks = &mut *TASKS.0.get();

        for i in 0..MAX_TASKS {
            if tasks[i].id == id {
                if tasks[i].state != TaskState::Empty && tasks[i].state != TaskState::Dead {
                    tasks[i].state = TaskState::Dead;
                    return true;
                }

                return false;
            }
        }
    }

    false
}

pub fn task_count() -> usize {
    unsafe {
        let tasks = &*TASKS.0.get();

        tasks
            .iter()
            .filter(|task| task.state != TaskState::Empty && task.state != TaskState::Dead)
            .count()
    }
}
