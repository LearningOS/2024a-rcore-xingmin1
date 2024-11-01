//! Process management syscalls
//!
use alloc::sync::Arc;
use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
};
use crate::config::PAGE_SIZE;
use crate::mm::{translated_byte_buffer, MapPermission, SimpleRange, VirtAddr};
use crate::task::{current_memory_set_mut, get_task_info, TaskControlBlock};
use crate::timer::get_time_us;

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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time",
        current_task().unwrap().pid.0
    );
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
    trace!(
        "kernel:pid[{}] sys_task_info",
        current_task().unwrap().pid.0
    );
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
    trace!(
        "kernel:pid[{}] sys_mmap",
        current_task().unwrap().pid.0
    );
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

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap",
        current_task().unwrap().pid.0
    );
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
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let new_task = Arc::new(TaskControlBlock::new(all_data.as_slice()));
        let current_task = current_task().unwrap();
        current_task.inner_exclusive_access().children.push(new_task.clone());
        new_task.inner_exclusive_access().parent = Some(Arc::downgrade(&current_task));
        let new_pid = new_task.pid.0;
        // add new task to scheduler
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
