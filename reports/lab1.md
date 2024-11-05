## 简单总结你实现的功能

### 实现功能

引入一个新的系统调用 `sys_task_info` 以获取当前任务的信息：

* 任务信息包括任务控制块相关信息（任务状态）、任务使用的系统调用及调用次数、系统调用时刻距离任务第一次被调度时刻的时长（单位ms）

### 实现过程

1. add some fields on TaskControlBlock for task\_info:

    * first\_schedule\_time,
    * syscall\_count
2. initialize new fields in TaskControlBlock

    * Add initialization for new fields in TaskControlBlock upon TASK\_MANAGER initialization.
    * Record the first scheduled time.
3. implement syscall count

    * Add functions to increment the current syscall count on syscall invocation.
    * invoke the function upon receiving a system call.
4. Implement sys\_task\_info syscall

    * Move TaskInfo struct from syscall to task module.
    * Add function to set task information.
5. correct initialization of first\_schedule\_time

    * Fix the initialization logic for `first_schedule_time`.
    * Introduce `FirstScheduleTime` enum for better handling of schedule time states.

## 简答作业[¶](https://learningos.cn/rCore-Camp-Guide-2024A/chapter3/5exercise.html#id4)

1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 [三个 bad 测例 (ch2b_bad_*.rs)](https://github.com/LearningOS/rCore-Tutorial-Test-2024A/tree/master/src/bin) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

    * [rustsbi] RustSBI version , adapting to RISC-V SBI v1.0.0

    1. 第一个 bad 测例：`ch2b_bad_1.rs`的错误行为： `(0x0 as *mut u8).write_volatile(0);`
        该地址非法，会导致内存访问错误，错误信息如下： `[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.`
    2. 第二个 bad 测例：`ch2b_bad_2.rs`的错误行为： `core::arch::asm!("sret");`
        该指令为 S 态特权指令，U 态无法执行，错误信息如下： `[kernel] IllegalInstruction in application, kernel killed it.`
    3. 第三个 bad 测例：`ch2b_bad_3.rs`的错误行为：

        ```rust
        let mut sstatus: usize;
        unsafe { 
            core::arch::asm!("csrr {}, sstatus", out(reg) sstatus); 
        }
        ```

        该指令为访问 S 态寄存器，U 态无法执行，错误信息如下： `[kernel] IllegalInstruction in application, kernel killed it.`
2. 深入理解 [trap.S](https://github.com/LearningOS/rCore-Camp-Code-2024A/blob/ch3/os/src/trap/trap.S) 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:

    1. L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景。

        1. 刚进入 `__restore` 时，`a0` 代表了要前往的应用的trap\_context的地址，即其内核栈栈顶。
        2. `__restore` 的两种使用情景：

            1. 每个任务第一次启动时，会在`__switch`中修改sp和ra寄存器的值，使得直接用`ret`跳转到`__restore`，用来启动一个应用。
            2. 任务切换时，是正常的`__alltraps`，`trap_handler`，`__restore`流程，可能会在`trap_handler`的最后切换trap控制流以达到经过`__restore`后切换任务的功能。
    2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

        ```asm
        ld t0, 32*8(sp)
        ld t1, 33*8(sp)
        ld t2, 2*8(sp)
        csrw sstatus, t0
        csrw sepc, t1
        csrw sscratch, t2 
        ```

        **​`sstatus`​**：保存trap前的中断使能状态和和特权级。

        **​`sepc`​**：用于指示trap返回后pc的位置，确保任务从正确的位置继续执行。

        **​`sscratch`​**：用于保存和恢复用户栈指针，在`__restore`最后与`sp`交换值，实现用户态与内核态之间的栈切换。
    3. L50-L56：为何跳过了 `x2` 和 `x4`？

        ```assembly
        ld x3, 3*8(sp)
        .set n, 5
        .rept 27
        LOAD_GP %n
        .set n, n+1
        .endr
        ```

        **​`x2`​**  **(**​**​`sp`​**​ **)** ：栈指针特殊处理，以避免与任务切换时的栈管理冲突。它在前面已经load到`sscratch`中了，在后面`sp`会与`sscratch`交换。

        **​`x4`​**  **(**​**​`tp`​**​ **)** ：application does not use it，前面没有在`__alltraps`中保存它。
    4. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

        ```
        csrrw sp, sscratch, sp
        ```

        `sp`是当前任务用户栈栈顶

        `sscratch`是当前任务内核栈栈顶
    5. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
        发生状态切换在最后一条指令`sret`，`sstatus`寄存器中的`spp`字段为User。
        执行`mret`指令后，处理器会根据`sstatus`中的设置，恢复到用户态。
    6. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

        ```
        csrrw sp, sscratch, sp
        ```

        `sp`-\>kernel stack,

        `sscratch`-\>user stack
    7. 从 U 态进入 S 态是哪一条指令发生的

        ```rust
        pub fn syscall(id: usize, args: [usize; 3]) -> isize {
            let mut ret: isize;
            unsafe {
                core::arch::asm!(
                    "ecall",
                    inlateout("x10") args[0] => ret,
                    in("x11") args[1],
                    in("x12") args[2],
                    in("x17") id
                );
            }
            ret
        }
        ```

        是在上面的`ecall`指令发生的。

## **荣誉准则**​[¶](https://learningos.cn/rCore-Camp-Guide-2024A/honorcode.html#id1)

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    > *null*
    >
2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    > *课本*
    >
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。