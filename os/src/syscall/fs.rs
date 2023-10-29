//! File and filesystem-related syscalls

// use crate::syscall::SYSCALL_WRITE;
// use crate::task::add_syscall_count;


const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    //每次调用都计数一下
    //TASK_MANAGER.inner 这种方式显然无法用，这个玩意儿是private的，尽量还是不要修改这些东西吧……
    // add_syscall_count(SYSCALL_WRITE);
    // 我直接扔到 mod 里，省的引用了

    trace!("kernel: sys_write");
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
