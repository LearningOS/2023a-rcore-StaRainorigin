//! Process management syscalls
// use alloc::task;

use crate::{
    // config::MAX_SYSCALL_NUM,
    config::PAGE_SIZE,
    // config::MEMORY_END,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,  current_user_token ,TaskInfo, get_current_task_info, create_memory_area, delete_memory_area,
    },
    timer::get_time_us, 
    mm::translated_va_to_pa, // mm::translated_byte_buffer // ,get_time_ms,
    // mm::create_framed_area,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,     // 存储秒数部分的时间值。
    pub usec: usize,    // 在某些情况下需要更精确的时间度量，例如微秒
}

/// Task information
/// 移走啦->task::task

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
/// 你（我）的任务：用秒和微秒获取时间
/// 提示：你可以用虚拟内存管理重新实现它。
/// 提示：如果[`TimeVal`]被分成两页怎么办？
/// 
/// 心路历程：
/// 一开始我以为是通过虚拟内存来求时间，我寻思，虚拟内存和时间有什么关系，这玩意儿还能求时间。我百思不得其解，虚拟内存管理，怎么管着管着出来个时间了？
/// 然后用ch3的方法实现了一下，好像也没问题啊，能跑通啊。
/// 直到我看到了 sys_task_info 任务注释和上面的话术一摸一样，我才知道，哦，我理解错了
/// 
/// 看来应该是这样的，_tz 原先传进来的是一个物理地址，但是在我们添加了虚拟内存机制之后，这个地址已经要被当作虚拟地址对待了
/// 所应该是把这个虚拟地址给转换成实际的物理地址
/// 
/// 转换的话，应该是需要用到 page_table.rs 中的 translate 方法 -> 就需要获得一个 PageTable -> 获取PageTable的方式是通过一个token
/// 
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {   //所以这个_tz到底是干啥用的
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    if let Some(ts) = translated_va_to_pa(current_user_token(), (ts as usize).into()) {
        let ts = ts.get_mut();
        // unsafe 在这里提示没用了？
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
            };
        0
    } else {
        -1
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
/// 你（我）的任务：完成 sys_task_info 以传递测试用例
/// 提示：你可以用虚拟内存管理来重新实现它。
/// 提示：如果 [`TaskInfo`] 被分成两页怎么办？
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    if let Some(ti) = translated_va_to_pa(current_user_token(), (ti as usize).into()) {
        let ti = ti.get_mut();
        *ti = get_current_task_info();
        0
    } else {
        -1
    }
    
}


// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if start%PAGE_SIZE==0 && port&!0x7==0 && port&0x7!=0 {
        create_memory_area(start, len, port)
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if start%PAGE_SIZE==0 && len%PAGE_SIZE==0 {
        delete_memory_area(start, len)
    } else {
        -1
    }

}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
