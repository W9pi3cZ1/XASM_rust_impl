# XASM1 dispatch fallthrough reproducer

这个目录给原版 XASM1 的作者/维护者复现一个最小 bug。

## 文件

- `temp.asm`：源码
- `disk.bin`：用原版 `original/XASM_V1/compile` 编译出的 ROM（10 字节）

## 源码

```asm
copy 0x00 0x00 0x01 0x00 0x00 0x00 0x00 0x23
exit
```

`copy dest src` 的源地址是 `0x00000023`，所以这条 `copy` 的**最后一个操作数字节是 `0x23`**。

ROM：

```text
22 00 00 01 00 00 00 00 23 2a
```

## 预期行为

一个正确的 VM 应该：

1. 执行 `copy 0x100, 0x23`；
2. 执行 `exit`；
3. 正常退出，无输出。

community rebuild 和本项目的 Rust VM 都是这样：

```text
exit=Exit
```

## 原版 XASM1 的实际行为

原版 `XASM1` 直接 SIGSEGV。

```bash
mkdir -p run && cp disk.bin run/disk.bin
cd run
/path/to/XASM1
# Segmentation fault
```

## 原因

原版 `main` 的 opcode dispatch 链不是完整的 `else if`：

```c
if (op == 0x22) {        // copy
    ... 读 8 字节操作数 ...
    local += 8;          // 停在最后一个操作数字节
    ... 执行 copy ...
}
// 没有 break / continue / else
if (memory[local] == 0x23) {   // goto
    ...
}
```

`copy` 执行完后，`local` 指向 `0x23`（源地址最低字节）。VM 把它当成 `goto` 指令继续执行：

- 读到的 4 字节操作数是 `0x2a`（exit）以及后面的 EOF 填充 `0xff`；
- 拼出地址 `0x2affffff`；
- 访问 `memory[0x2affffff]`，越界段错误。

## 建议修复

给 dispatch 的每个 case 加上正确的 `break`/`continue`，或者在每条指令处理完后直接跳到循环末尾，确保不会用已经递增过的 `local` 继续匹配后面的 opcode。

受影响的不只是 `copy`：

- `geta`（0x24）之后；
- `>`（0x25）/ `<`（0x26）条件跳转之后；
- 任何读取操作数后 `local` 已变化、却继续 fallthrough 的 case。

community rebuild 使用 `switch`/正确的分支结构，没有这个问题；本仓库另外修正了 community 的 `not`/`lm`/`rm` 语义，使其与原版一致。

## 参考

- [../docs/xasm1-dispatch-bug.md](../docs/xasm1-dispatch-bug.md)
- [../docs/xasm1-decompiled.c](../docs/xasm1-decompiled.c)
