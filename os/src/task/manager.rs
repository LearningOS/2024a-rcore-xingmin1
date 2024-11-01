//!Implementation of [`TaskManager`]
use super::{TaskControlBlock};
use crate::sync::UPSafeCell;
use alloc::collections::{BinaryHeap};
use alloc::sync::Arc;
use core::cmp::Reverse;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    // ready_queue: VecDeque<Arc<TaskControlBlock>>,
    ready_priority_queue: BinaryHeap<Reverse<Arc<TaskControlBlock>>>
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_priority_queue: BinaryHeap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_priority_queue.push(Reverse(task));
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_priority_queue.pop().map(|Reverse(task)| task)
    }
    
    /// Peek the next process in the ready queue
    pub fn peek(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_priority_queue.peek().map(|Reverse(task)| task.clone())
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// Peek the next process in the ready queue
pub fn peek_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::peek_task");
    TASK_MANAGER.exclusive_access().peek()
}
