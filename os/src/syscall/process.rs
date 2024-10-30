//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    mm::translated_byte_buffer,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::get_time_us,
};
use crate::task::get_task_info;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

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
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let tv_size = core::mem::size_of::<TimeVal>();
    let buffers = translated_byte_buffer(current_user_token(), ts as *const u8, tv_size);
    let temp_tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let mut tv_slice = unsafe {
        core::slice::from_raw_parts(
            &temp_tv as *const _ as *const u8,
            tv_size,
        )
    };
    for buffer in buffers {
        buffer.copy_from_slice(&tv_slice[..buffer.len()]);
        tv_slice = &tv_slice[buffer.len()..];
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let ti_size = core::mem::size_of::<TaskInfo>();
    let task_info_temp = get_task_info();
    let mut task_info_slice = unsafe {
        core::slice::from_raw_parts(&task_info_temp as * const _ as * const u8, ti_size)
    };
    let buffers = translated_byte_buffer(current_user_token(), ti as *const u8, ti_size);
    for buffer in buffers {
        buffer.copy_from_slice(&task_info_slice[..buffer.len()]);
        task_info_slice = &task_info_slice[buffer.len()..];
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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
