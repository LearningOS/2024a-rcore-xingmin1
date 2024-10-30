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
use crate::config::PAGE_SIZE;
use crate::mm::{MapPermission, SimpleRange, VirtAddr};
use crate::task::{current_memory_set_mut, get_task_info};

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
    trace!("kernel: sys_task_info");
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

// start 没有按页大小对齐
// port & !0x7 != 0 (port 其余位必须为0)
// port & 0x7 = 0 (这样的内存无意义)
// [start, start + len) 中存在已经被映射的页
// 物理内存不足
    fn validate_mmap_params(start: usize, port: usize) -> Result<(), isize> {
        if start % PAGE_SIZE != 0 {
            debug!("start is not aligned by page size");
            return Err(-1);
        }
        if port & !0x7 != 0 {
            debug!("the rest of port must be 0");
            return Err(-1);
        }
        if port & 0x7 == 0 {
            debug!("this memory is meaningless");
            return Err(-1);
        }
        Ok(())
    }
// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if let Err(err) = validate_mmap_params(start, port) {
        return err;
    }
    let memory_set = current_memory_set_mut();
    let (start_va, end_va) = (VirtAddr::from(start), VirtAddr::from(start + len));
    let range = SimpleRange::new(start_va.floor(), end_va.ceil());
    if memory_set.map_area_overleap(range) {
        debug!("overleap with existing mapped area");
        return -1;
    }
    let mut permission = MapPermission::U;
    if port & 1 == 1{
        permission |= MapPermission::R;
    }  
    if port >> 1 & 1 == 1 {
        permission |= MapPermission::W;
    }
    if port >> 2 & 1 == 1 {
        permission |= MapPermission::X;
    }
    memory_set.insert_framed_area(start_va, end_va, permission);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start % PAGE_SIZE != 0 {
        debug!("start is not aligned by page size");
        return -1;
    }

    let memory_set = current_memory_set_mut();
    if memory_set.remove_framed_areas(VirtAddr(start).floor(), VirtAddr(start + len).ceil()) {
        0
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
