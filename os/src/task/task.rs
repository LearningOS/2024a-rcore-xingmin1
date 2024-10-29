//! Types related to task management

use alloc::collections::btree_map::BTreeMap;

use super::TaskContext;

/// The task control block (TCB) of a task.
#[derive(Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// The time when the first schedule occurs
    pub first_schedule_time: FirstScheduleTime,
    /// Array to count the number of each type of syscall
    pub syscall_count: BTreeMap<usize, u32>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum FirstScheduleTime {
    Undefined,
    Ms(usize)
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
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
