## 编程作业[¶](https://learningos.cn/rCore-Camp-Guide-2024A/chapter5/4exercise.html#id1 "永久链接至标题")

### 进程创建

大家一定好奇过为啥进程创建要用 fork + exec 这么一个奇怪的系统调用，就不能直接搞一个新进程吗？ 思而不学则殆，我们就来试一试！这章的编程练习请大家实现一个完全 DIY 的系统调用 spawn，用以创建一个新进程。

spawn 系统调用定义( [标准spawn看这里](https://man7.org/linux/man-pages/man3/posix_spawn.3.html) )：

```
fn sys_spawn(path: *const u8) -> isize
```

* syscall ID: 400
* 功能：新建子进程，使其执行目标程序。
* 说明：成功返回子进程id，否则返回 -1。
* 可能的错误：* 无效的文件名。

  * 进程池满/内存不足等资源错误。

TIPS：虽然测例很简单，但提醒读者 spawn **不必** 像 fork 一样复制父进程的地址空间。

> 回答
>
> 先用文件名得到app的elf数据，然后据此新建一个tcb，然后设置好父子关系，最后将其加入等待队列

### stride 调度算法

ch3 中我们实现的调度算法十分简单。现在我们要为我们的 os 实现一种带优先级的调度算法：stride 调度算法。

算法描述如下:

(1) 为每个进程设置一个当前 stride，表示该进程当前已经运行的“长度”。另外设置其对应的 pass 值（只与进程的优先权有关系），表示对应进程在调度后，stride 需要进行的累加值。

2. 每次需要调度时，从当前 runnable 态的进程中选择 stride 最小的进程调度。对于获得调度的进程 P，将对应的 stride 加上其对应的步长 pass。
3. 一个时间片后，回到上一步骤，重新调度当前 stride 最小的进程。

可以证明，如果令 P.pass \= BigStride / P.priority 其中 P.priority 表示进程的优先权（大于 1），而 BigStride 表示一个预先定义的大常数，则该调度方案为每个进程分配的时间将与其优先级成正比。证明过程我们在这里略去，有兴趣的同学可以在网上查找相关资料。

其他实验细节：

* stride 调度要求进程优先级 ≥2，所以设定进程优先级 ≤1 会导致错误。
* 进程初始 stride 设置为 0 即可。
* 进程初始优先级设置为 16。

为了实现该调度算法，内核还要增加 set\_prio 系统调用

```
// syscall ID：140
// 设置当前进程优先级为 prio
// 参数：prio 进程优先级，要求 prio >= 2
// 返回值：如果输入合法则返回 prio，否则返回 -1
fn sys_set_priority(prio: isize) -> isize;
```

实现 tips:

* 你可以在TCB加入新的字段来支持优先级等。
* 为了减少整数除的误差，BIG\_STRIDE 一般需要很大，但为了不至于发生反转现象（详见问答作业），或许选择一个适中的数即可，当然能进行溢出处理就更好了。
* stride 算法要找到 stride 最小的进程，使用优先级队列是效率不错的办法，但是我们的实验测例很简单，所以效率完全不是问题。事实上，很推荐使用暴力扫一遍的办法找最小值。
* 注意设置进程的初始优先级。

注意

为了让大家能在本编程作业中使用 `Vec` 等数据结构，我们利用第三方库 `buddy_system_allocator`为大家实现了堆内存分配器，相关代码位于 `mm/heap_allocator` 模块。

背景知识： [Rust 中的动态内存分配](https://rcore-os.github.io/rCore-Tutorial-Book-v3/chapter4/1rust-dynamic-allocation.html)

> 回答
>
> 新建结构体Stride和StrideSchedule
>
> 将等待队列的实现由VecDeque换为BinaryHeap，并更新new，add和fetch的实现
>
> 在调度任务时，增加相应的stride

### 实验要求

* 实现分支：ch5。
* 实验目录请参考 ch3。注意在reports中放入lab1-3的所有报告。
* 通过所有测例。
  在 os 目录下 `make run BASE=2` 加载所有测例， `ch5_usertest` 打包了所有你需要通过的测例， 你也可以通过修改这个文件调整本地测试的内容, 或者单独运行某测例来纠正特定的错误。 `ch5_stride`检查 stride 调度算法是否满足公平性要求，六个子程序运行的次数应该大致与其优先级呈正比，测试通过标准是maxruntimesprio/minruntimesprio\<1.5.
  CI 的原理是用 `ch5_usertest` 替代 `ch5b_initproc` ，使内核在所有测例执行完后直接退出。
  从本章开始，你的内核必须前向兼容，能通过前一章的所有测例。

## 问答作业

### stride 算法深入

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

#### 实际情况是轮到 p1 执行吗？为什么？

在使用 8bit 无符号整型储存 stride 时，`BIG_STRIDE` 设为 255。当 `p1.stride = 255`，`p2.stride = 250` 时，`p2` 执行一个时间片后，`p2.stride` 会增加其 `pass` 值（假设为10），导致 `p2.stride = 250 + 10 = 260`。由于 `stride` 是8位无符号整型，`p2.stride` 会发生溢出，计算为 `260 - 256 = 4`。

因此，`p1.stride = 255`，`p2.stride = 4`，下次调度时会选择 `p2` 执行。

#### 我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。为什么？尝试简单说明（不要求严格证明）。

1. 假设在第n次调度时，STRIDE_MAX<sub>n</sub> - STRIDE_MIN<sub>n</sub> < BigStride / 2

    则，本次调度后（第n+1次调度时）

    $$
    STRIDE\_MAX_{n+1} - STRIDE\_MIN_{n+1} <= max(STRIDE\_MAX_n, STRIDE\_MIN_n + BigStride / 2) - STRIDE\_MIN_n <= BigStride / 2
    $$

    即第n+1次调度时，STRIDE_MAX<sub>n</sub> - STRIDE_MIN<sub>n</sub> < BigStride / 2
2. 已知第0次调度时，STRIDE_MAX<sub>n</sub> - STRIDE_MIN<sub>n</sub> = 0
3. 由1、2推得：在所有调度时，都满足STRIDE_MAX<sub>n</sub> - STRIDE_MIN<sub>n</sub> < BigStride / 2
4. 在调度之间可以插入任务，只要在插入任务后使任务集合不违反STRIDE_MAX<sub>n</sub> - STRIDE_MIN<sub>n</sub> < BigStride / 2即可。

#### 已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。

```rust
    use core::cmp::Ordering;

    struct Stride(u64);

    impl PartialOrd for Stride {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            // ...
        }
    }

    impl PartialEq for Stride {
        fn eq(&self, other: &Self) -> bool {
            false
        }
    }
```

```rust

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { 
            let diff: i128 = self.0 as i128 - other.0 as i128;
	        let order = if diff < - (BIG_STRIDE as i128 / 2) {
	            Ordering::Greater
	        } else if diff < 0 {
	            Ordering::Less
	        } else if diff == 0 {
	            Ordering::Equal
	        } else if diff <= BIG_STRIDE as i128 / 2 {
	            Ordering::Greater
	        } else {
	            Ordering::Less
	        }
			Some(order)
        }
    }
}

```

TIPS: 使用 8 bits 存储 stride, BigStride = 255, 则: (125 < 255) == false, (129 < 255) == true.

# **荣誉准则**

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > ChatGPT 关于rust语法方面的内容
2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > 课本
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
