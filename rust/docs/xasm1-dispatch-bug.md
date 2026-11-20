# XASM1 的 dispatch bug：为什么大型程序会随机崩溃

这份记录基于 Ghidra 12.2 对原始 `original/XASM_V1/XASM1` 的反编译（`main` 约 425 行 C 伪代码）以及 gdb 的单步验证。完整反编译输出在 [xasm1-decompiled.c](xasm1-decompiled.c)。

## 结论

原版 `XASM1` 的 opcode dispatch 链不是完整的 `else if` 链，等价于 `switch` 里漏了 `break`、或者手写 `if` 链漏了 `else`。某些指令执行完后，`local` 已经指向了新的位置，但代码会继续用这个新 `local` 去匹配后面的 opcode。最严重的是：

- `copy`（0x22）
- `geta`（0x24）
- `>`（0x25）/ `<`（0x26）

`copy` 读取 8 字节操作数后只把 `local` 加了 8，因此 `local` 指向**最后一个操作数字节**，而不是下一条指令。随后 VM 会检查这个字节是否等于 `0x23`、`0x24`……如果等于，就会把操作数当成指令执行，PC 直接跑飞。

最小可分发复现：[../repro/temp.asm](../repro/temp.asm) + [../repro/disk.bin](../repro/disk.bin)。

## 一个具体复现

```asm
; 0x12: copy 0x124, 0x125
12: 22 00 00 01 24 00 00 01 25
```

`copy` 的最后一个操作数字节是 `0x25`，而 `0x25` 是 `>` 的 opcode。原版 VM 在执行完 copy 后：

1. `local` 从 0x12 变成 0x1a（最后一个操作数字节）；
2. 继续检查 `memory[0x1a] == 0x26`、`== 0x27`……最终匹配 `0x25`；
3. 把 `local=0x1a` 当成 `>` 指令，读取错位的 12 字节操作数；
4. `>` 条件成立，`local = target - 1`，跳到 0x27（另一条指令的操作数中间）；
5. 段错误。

gdb 里可以直接看到 guest PC 跳到操作数中间，native `putn`/`goto`/`>` 读到 `0x03000004`、`0x21000005` 这类由操作数字节拼出来的假地址。

## 为什么是“布局敏感”

是否崩溃取决于：

- `copy` 的源地址最低字节；
- `geta` 的目标地址最低字节；
- `>`/`<` 跳转目标前一个字节；
- 以及这些字节是否恰好等于后续 dispatch 检查的 opcode。

所以同一份源码只改几个无用变量、改变数据地址，就可能从正常运行变成段错误。这不是 ROM 非法，也不是随机数，而是原版 VM 自己的 dispatch bug。

## 工作区里的规避措施

### XPL 编译器

1. **条件跳转 NOP 填充**
   - 每个 `>`/`=` 后面插入 `0x00`；
   - 每个控制流 label 前、后各插入 `0x00`。
   - 这样原版 VM 在“目标 - 1”或“下一条”位置看到的是 NOP，而不是 `0x26`/`0x27`/`0x28`。

2. **变量/临时变量 256 字节对齐**
   - XPL 的变量/临时 cell 全部放在 256 字节边界上（每个 cell 后填充 255 个 0）。
   - 因此 `copy dest, src` 的源地址最低字节恒为 `0x00`，原版 VM 的 copy fallthrough 会看到 NOP，不会误执行。
   - 代价是 ROM 变大（例如 `memedit` 约 44 KB），但仍远低于原版 1 MiB 内存上限。

3. **`peek` 使用自修改 `lm` + restore**
   - 不再 patch `copy` 的源地址（那是原版 bug 的触发点），而是 patch `lm` 的第一个操作数。
   - `lm` 的第二个操作数始终是 256 字节对齐的 cell，最后一个操作数字节为 `0x00`，不会触发 dispatch fallthrough。
   - `lm` 读取目标 cell 后会把目标清零，所以紧接着用第二组 `lm` 把值写回去，逻辑上是非破坏性读取。
   - `memedit` 的 `j` 两次（addr=0x20）以及更高地址现在都不会崩溃。

### community rebuild

community 的 `vm.c` 使用 `switch`/正确的 `else if` dispatch，不会有这个 fallthrough 问题。它原本还有两处与原版不一致：

- `not`：community 是 2 操作数逻辑非，原版是 3 操作数 `dest = (a != b)`；
- `lm/rm`：community 用 `*a ± *b`，原版用 `address_of_a ± *b`。

本仓库已经把 community C 实现修正为原版语义，并更新了 `docs/code_table.md`。

## 验证

- `primes.xpl`（嵌套循环）、`countdown.xpl`、`fizzbuzz.xpl`、`fib.xpl` 在原始 `XASM1` 上输出正确。
- `memedit.xpl` 在原始 `XASM1` 上可交互运行（alt screen + 补零 + `q` 退出）。
- `cargo test --workspace`：56 个测试通过。
