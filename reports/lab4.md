## 编程作业

### 硬链接

硬链接要求两个不同的目录项指向同一个文件，在我们的文件系统中也就是两个不同名称目录项指向同一个磁盘块。

本节要求实现三个系统调用 `sys_linkat、sys_unlinkat、sys_stat` 。

**linkat**：

> * syscall ID: 37
> * 功能：创建一个文件的一个硬链接， [linkat标准接口](https://linux.die.net/man/2/linkat) 。
> * Ｃ接口： `int linkat(int olddirfd, char* oldpath, int newdirfd, char* newpath, unsigned int flags)`
> * Rust 接口： `fn linkat(olddirfd: i32, oldpath: *const u8, newdirfd: i32, newpath: *const u8, flags: u32) -> i32`
> * 参数：* olddirfd，newdirfd: 仅为了兼容性考虑，本次实验中始终为 AT\_FDCWD (-100)，可以忽略。
>
>   * flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。
>   * oldpath：原有文件路径
>   * newpath: 新的链接文件路径。
> * 说明：* 为了方便，不考虑新文件路径已经存在的情况（属于未定义行为），除非链接同名文件。
>
>   * 返回值：如果出现了错误则返回 -1，否则返回 0。
> * 可能的错误* 链接同名文件。

> 回答
>
> 函数的作用是在虚拟文件系统（VFS）中为现有文件创建一个新的硬链接。下面是对该函数实现的详细说明：
>
> 1. **获取文件系统锁**：
>     确保在操作期间，文件系统处于安全的状态，防止并发修改。
> 2. **定义操作闭包** op：
>
>    * **检查根 inode 是否为目录**：使用 assert! 确保当前的根 inode 是一个目录。
>    * **查找旧文件的 inode ID**：调用 `self.find_inode_id` 方法，在根目录下查找名为 old_name 的文件，获取其 inode ID。
> 3. **读取旧文件的 inode ID**：
     >
     >     * 使用 `self.read_disk_inode` 方法执行之前定义的操作闭包 op。
>     * 如果找到了文件的 inode ID，继续执行；否则，返回 `false`，表示操作失败。
> 4. **增加旧文件的链接计数**：
     >
     >     * 获取旧文件 inode 在磁盘上的位置（块 ID 和偏移量）。
>     * 通过块缓存获取对应的磁盘块，并对其进行修改。
>     * 在修改闭包中，将 inode 的链接计数 `nlink` 加一，表示新增了一个硬链接。
> 5. **在目录中添加新的目录项**：
     >
     >     * **计算当前目录的文件数量**：通过目录大小除以目录项大小 DIRENT_SZ，得到已有的文件数量 file_count。
>     * **更新目录大小**：计算新的目录大小 new_size，并调用 `self.increase_size` 方法增加根 inode 的大小。
>     * **创建新的目录项**：使用 DirEntry::new 创建新的目录项，名称为 new_name，关联的 inode ID 为旧文件的 inode ID。
>     * **写入新的目录项**：使用 root_inode.write_at 方法，将新的目录项写入目录的末尾。
> 6. **同步缓存并释放锁**：
     >
     >     * 调用 block_cache_sync_all 将所有缓存的修改写回磁盘，确保数据的一致性。
>     * 返回 `true`，表示硬链接创建成功。
>     * 文件系统的锁将在函数结束时自动释放。
>
> 总体而言，linkat 函数实现了在文件系统中为指定的文件创建一个新的硬链接。通过增加目标文件的链接计数并在目录中添加新的目录项来实现这一功能。

**unlinkat**:

> * syscall ID: 35
> * 功能：取消一个文件路径到文件的链接, [unlinkat标准接口](https://linux.die.net/man/2/unlinkat) 。
> * Ｃ接口： `int unlinkat(int dirfd, char* path, unsigned int flags)`
> * Rust 接口： `fn unlinkat(dirfd: i32, path: *const u8, flags: u32) -> i32`
> * 参数：* dirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。
    >
    >   * flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。
>   * path：文件路径。
> * 说明：* 注意考虑使用 unlink 彻底删除文件的情况，此时需要回收inode以及它对应的数据块。
> * 返回值：如果出现了错误则返回 -1，否则返回 0。
> * 可能的错误* 文件不存在。

> 回答
>
> 1. **锁定文件系统**：首先，通过 `self.fs.lock()` 锁定文件系统，确保操作的原子性和线程安全。
> 2. **查找目标文件的 inode_id**：
     >
     >     * 如果未找到对应文件，返回 `false`。
> 3. **修改文件节点的链接计数**：
     >
     >     * 使用文件系统的 `get_disk_inode_pos` 方法获取目标文件节点的位置，包括块编号 inode_block_id 和块内偏移 inode_block_offset。
>     * 锁定对应的块缓存，调用 `modify` 方法修改 `DiskInode` 结构体：
        >
        >       * 将链接计数 `nlink` 减一。
>       * 如果 `nlink` 减至 0，表示没有其他硬链接引用该文件，调用新定义的 `Inode::clear_disk_inode` 方法清除节点数据。
> 4. **删除目录项**：
     >
     >     * 调用 `self.modify_disk_inode` 方法修改当前目录节点：
             >
             >       * 计算目录中的文件数量 file_count。
>       * 遍历目录项，使用 `DiskInode::read_at` 方法读取每个目录项，查找名称为 name 的目录项。
>       * 找到目标目录项后，使用 `DiskInode::write_at` 方法将该目录项的数据清零，表示删除该目录项。
> 5. **同步缓存并返回**

**fstat**:

> * syscall ID: 80
> * 功能：获取文件状态。
> * Ｃ接口： `int fstat(int fd, struct Stat* st)`
> * Rust 接口： `fn fstat(fd: i32, st: *mut Stat) -> i32`
> * 参数：* fd: 文件描述符
    >
    >   * st: 文件状态结构体
    >
    >   ```
    >   #[repr(C)]
    >   #[derive(Debug)]
    >   pub struct Stat {
    >       /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
    >       pub dev: u64,
    >       /// inode 文件所在 inode 编号
    >       pub ino: u64,
    >       /// 文件类型
    >       pub mode: StatMode,
    >       /// 硬链接数量，初始为1
    >       pub nlink: u32,
    >       /// 无需考虑，为了兼容性设计
    >       pad: [u64; 7],
    >   }
    >
    >   /// StatMode 定义：
    >   bitflags! {
    >       pub struct StatMode: u32 {
    >           const NULL  = 0;
    >           /// directory
    >           const DIR   = 0o040000;
    >           /// ordinary regular file
    >           const FILE  = 0o100000;
    >       }
    >   }
    >   ```

> 回答
>
> 通过文件描述符 fd 获取对应文件的状态信息，并将结果写入用户提供的内存位置 st。
>
> 以下是 sys_fstat 函数的实现步骤：
>
> 1. **获取当前任务和文件描述符表**：
     >
     >     通过 current_task() 获取当前运行的任务（进程）。
> 2. **检索文件对象**：
     >
     >     * 使用文件描述符 fd 从当前任务的文件描述符表 `fd_table` 中获取对应的文件对象。
>     * 如果文件描述符无效，返回 `-1`，表示出错。
> 3. **获取文件状态信息**：
     >
     >     调用文件对象的 stat() 方法，获取文件的状态信息，返回一个 Stat 结构体，其中包含文件大小、权限等信息。
> 4. **准备将状态信息复制到用户空间**：
     >
     >     * 计算 Stat 结构体的大小 st_size。
>     * 将 Stat 结构体转换为字节切片 stat_slice，以便后续复制。
> 5. **转换用户空间指针，并复制数据**：
     >
     >     * 使用 translated_byte_buffer 将用户空间的指针 st 转换为内核可访问的缓冲区数组 buffers。
>     * 遍历缓冲区数组，将文件状态信息逐一复制到用户提供的内存区域。




### 实验要求

* 实现分支：ch6。
* 实验目录要求不变。
* 通过所有测例。
  在 os 目录下 `make run BASE=2` 加载所有测例， `ch6_usertest` 打包了所有你需要通过的测例，你也可以通过修改这个文件调整本地测试的内容。
  你的内核必须前向兼容，能通过前一章的所有测例。

## 问答作业

1. 在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？

root inode 是一个 `Inode` 结构体的实例，负责存储根目录的元数据和对子文件或目录的引用。root inode是整个文件系统的起点，代表文件系统的根目录，起着根索引、文件系统在内核中锚点的作用，如果root inode内容损坏，则无法正常在根目录创建、访问文件，也就相当于文件系统完全无法使用，而很多功能（如app的加载）需要文件系统，无法使用，可能会导致系统崩溃。