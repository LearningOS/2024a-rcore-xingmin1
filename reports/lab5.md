## 问答作业

1. 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 -
   需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

   **需要回收的资源有哪些？**

    1. **线程的内核栈**
    2. **线程的用户栈**
    3. **线程的任务控制块**
    4. **进程的虚拟内存空间**
    5. **文件描述符表**
    6. **同步原语资源**：如信号量、互斥量、条件变量，以及等待队列。
    7. **系统定时器队列中的本进程的线程**。

   **其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？**

    1. **任务调度器的就绪队列**：

        * 需要。
        * **原因**：这些线程等待被 CPU 调度执行，进程退出时，应从就绪队列中移除这些线程的 TCB，并回收其资源，防止它们被再次调度。
    2. **同步原语的等待队列**（如信号量、互斥量、条件变量）：

        * 不需要。
        * **原因**：最后进程彻底结束后可以统一回收，因为进程处于僵尸状态时，不会再其调度这个进程真的等待队列中的线程。
    3. **定时器队列**：

        * 需要。
        * **原因**：防止定时器到期后访问已退出的线程。
    4. **进程的线程列表**：

        * 不需要。
        * **原因**：进程彻底退出后，线程列表会被自动释放。
2. 对比以下两种 `Mutex.unlock` 的实现，二者有什么区别？这些区别可能会导致什么问题？

    ```rust
    impl Mutex for Mutex1 {
        fn unlock(&self) {
            let mut mutex_inner = self.inner.exclusive_access();
            assert!(mutex_inner.locked);
            mutex_inner.locked = false;
            if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
                add_task(waking_task);
            }
        }
    }

    impl Mutex for Mutex2 {
        fn unlock(&self) {
            let mut mutex_inner = self.inner.exclusive_access();
            assert!(mutex_inner.locked);
            if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
                add_task(waking_task);
            } else {
                mutex_inner.locked = false;
            }
        }
    }
    ```

   **锁释放的时机不同**：

    * **Mutex1**：无论是否有等待的任务，都会立即将 `locked` 设置为 `false`。
    * **Mutex2**：只有在没有等待任务的情况下，才将 `locked` 设置为 `false`。

   在按照前rcore的实现中，lock操作在阻塞结束后，会不再加锁。Mutex1会导致不持有锁而进入临界区

   假如lock操作在阻塞结束后，加锁（即set为true）。当有等待任务时，Mutex1 先释放锁，再唤醒等待任务。这可能导致在等待任务被唤醒并调度之前，其他任务先获取到锁，然后重复set
   locked为true，导致临界区中可能同时存在多个线程。