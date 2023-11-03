//! Types related to task management


use super::TaskContext;
use crate::config::TRAP_CONTEXT_BASE;
use crate::config::MAX_SYSCALL_NUM;
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
// use crate::timer::get_time;
use crate::trap::{trap_handler, TrapContext};

/// The task control block (TCB) of a task.
pub struct TaskControlBlock {
    /// Save task context
    pub task_cx: TaskContext,

    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,

    /// Application address space
    pub memory_set: MemorySet,

    /// The phys page number of trap context
    pub trap_cx_ppn: PhysPageNum,

    /// The size(top addr) of program which is loaded from elf file
    pub base_size: usize,

    /// Heap bottom
    pub heap_bottom: usize,

    /// Program break
    pub program_brk: usize,

    /// 计数
    pub syscall_counts: [u32; MAX_SYSCALL_NUM],

    /// 记录起始时间
    pub time_lastcall: usize,

}

impl TaskControlBlock {
    /// get the trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    /// get the user token  token通常为唯一标识符
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// Based on the elf info in program, build the contents of task in a new address space
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            syscall_counts: [0u32; MAX_SYSCALL_NUM],
            time_lastcall: 0,

        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

/// 
///这段代码是一个 Rust 结构 TaskControlBlock 的定义，其中包括了一些成员字段和 TaskControlBlock 结构的实现，我将着重分析 new 方法。
// new 方法的作用是创建一个新的 TaskControlBlock 实例，用于表示一个任务（或进程）的控制块。这个控制块包含了与任务执行相关的各种信息，包括任务的内存空间、上下文信息、堆栈、系统调用计数等。
// 以下是 new 方法的主要步骤和作用：
// MemorySet::from_elf(elf_data)：这一行创建了一个 MemorySet 对象，它是用来管理任务的内存空间的。from_elf 方法根据 ELF 文件的数据（elf_data）构建了任务的内存布局，包括程序头、trampoline、陷阱上下文和用户堆栈等。
// trap_cx_ppn：这一行通过在内存空间中查找陷阱上下文（TrapContext）的物理页号（ppn）来获得陷阱上下文的位置。陷阱上下文用于保存任务在内核态执行时的上下文信息。
// TaskStatus::Ready：创建一个 TaskStatus 枚举实例，表示任务的状态为 "Ready"，即任务准备好执行。
// kernel_stack_position(app_id)：计算内核堆栈的起始地址和结束地址，然后将这个区域映射到内核空间。
// 创建 TaskContext：创建一个任务上下文（TaskContext）实例，该上下文用于任务在内核态执行时的初始状态。
// 构建 TaskControlBlock：使用上述信息创建一个 TaskControlBlock 实例，包括任务状态、任务上下文、内存布局、陷阱上下文位置、程序的大小（base_size）、堆底（heap_bottom）、程序断点（program_brk）、系统调用计数、以及任务的启动时间。
// 准备 TrapContext：通过 task_control_block.get_trap_cx() 获取任务的陷阱上下文，并初始化它以便任务从用户态切换到内核态时能够正确处理陷阱和系统调用。
// 最终，new 方法返回一个包含了任务控制信息的 TaskControlBlock 实例，该实例准备好用于执行一个程序，其中包括了程序的内存布局、初始状态和执行环境。这在操作系统中用于创建和管理进程或任务。


    /// change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}

#[allow(dead_code)]
/// crate::task
/// 没有说明就给他一个说明！
#[derive(Clone)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],        //改成usize，能根据系统弹性分配一下长度？可能更好？       //题目给的u32，改回去，怂了
    /// Total running time of task
    pub time: usize,
}
