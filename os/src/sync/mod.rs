//! Synchronization and interior mutability primitives

mod condvar;
mod mutex;
mod semaphore;
mod up;

use alloc::vec;
use alloc::vec::Vec;
pub use condvar::Condvar;
pub use mutex::{Mutex, MutexBlocking, MutexSpin};
pub use semaphore::Semaphore;
pub use up::UPSafeCell;

/// Detect deadlock
pub fn deadlock_detect(available: &mut [isize], allocation: &[Vec<usize>], need: &[Vec<usize>]) -> bool {
    debug!("enter deadlock_detect");
    debug!("ava: {available:#?}");
    debug!("alloc: {allocation:#?}");
    debug!("need:");
    let task_len = allocation.len();
    let mut finish = vec![false; task_len];
    let mut could_change = true;
    debug!("enter loop. task_len: {task_len}");
    while could_change {
        for task_id in 0..task_len {
            debug!("task_id: {task_id}");
            if finish[task_id] {
                if task_id == task_len - 1 {
                    could_change = false;
                    break;
                }
                continue;
            }
            if need[task_id].iter()
                .zip(available.iter())
                .all(|(need, ava)| *need as isize <= *ava)
            {
                finish[task_id] = true;
                for (ava, allo) in available.iter_mut().zip(allocation[task_id].iter()) {
                    *ava += *allo as isize;
                }
                break;
            }
            if task_id == task_len - 1 {
                could_change = false;
            }
        }
    }
    debug!("leave deadlock_detect");
    finish.iter().any(|x| !*x)
}