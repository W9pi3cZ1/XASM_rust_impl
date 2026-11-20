# XASM 兼容性说明与逆向记录

本文记录本实现与原始 `XASM_V1`、社区重建版之间的边界，以及为什么 community 扩展指令不适合“自动降级”到原始 VM。

## 1. 一句话结论

- **ROM 字节格式**：原始编译器、community 编译器、本实现三者一致。
- **原始 23 条指令 + `0xff`/`sto`**：本实现生成的 ROM 可被原始 `XASM1` 原样执行。
- **community 位运算 `0x41..0x46`**：原始 VM 没有这些 opcode，无法执行。本实现不尝试不可靠的降级，而是在 `--target legacy` 下精确报错。

## 2. 原始编译器 `compile`

原始文件：`../XASM_Community_Rebuild/original/XASM_V1/compile`（ELF x86-64，未 strip；符号里有 `XASM_IDE_V1.c`）。

### 2.1 固定路径

`main` 直接：

```c
fp  = fopen("./temp.asm", "r");
out = fopen("./disk.bin", "w");
```

没有命令行参数，所以本实现提供 `--legacy` 复刻这个工作流。

### 2.2 字符流解析，而不是 token 流

原始编译器逐字符读取 `cache`，按首字符进入一条很长的 `if` 链：

- `//`：跳过并回显到行尾。
- `"`：翻转 `local`（字符串模式）。字符串模式下字节原样写入，**不处理 `\n`、`\t`、`\xHH`**。
- `0xHH`：读 `x` 后恰好再读两个字符，`strtol(hex, NULL, 16)`，然后 `fputc`。
- 助记符：按首字母链式匹配，例如
  - `a` -> `d` -> `d` = `add` (0x01)
  - `a` -> `n` -> `d` = `and` (0x11)
  - `n` -> `o` -> `t` = `not` (0x13)
  - `g` -> `o` -> `t` -> `o` = `goto` (0x23)
  - `g` -> `e` -> `t` -> `a` = `geta` (0x24)
  - `p` -> `u` -> `t` -> `c` = `putc` (0x31)
  - 等等

由此产生的原始行为：

- 没有 token 边界：`add0x01` 会先匹配 `add`，再解析 `0x01`。
- 没有 label/`@` 概念。
- 未知字符静默忽略。
- `0x1` 后只读一个字符，若下一字符是空格，会连空格一起吃掉。
- 字符串里不能写转义；反斜杠就是反斜杠。

本实现的默认词法更严格（token 化），但通过差分测试保证**对合法原始程序**输出逐字节相同。

### 2.3 指令编码

每条指令 = 1 字节 opcode + N 个 4 字节操作数，操作数按**大端序**书写。例如：

```asm
mov 0x00 0x00 0x01 0x00 0x00 0x00 0x00 0x48
```

编码为：

```
21 00 00 01 00 00 00 00 48
```

`mov` 的第 2 个操作数是立即数 `0x00000048`，不是地址解引用。

## 3. 原始 VM `XASM1`

原始文件：`../XASM_Community_Rebuild/original/XASM_V1/XASM1`。

### 3.1 内存与启动

- 固定打开 `./disk.bin`。
- 全局 `int memory[0xffffe + 1]`，即 1,048,575 个 32 位单元（约 4 MiB）。
- 启动时逐字节 `fseek + fgetc` 填 `memory[0..=0xffffe]`。这就是原版启动约 4–5 秒的原因。
- EOF 之后 `fgetc` 返回 `-1`，所以 **ROM 之后的所有单元都是 `-1`，不是 0**。Rust VM 默认复刻这一点；`--zero-fill` / `--community` 改为 0。
- `PC = 0`。**没有入口地址头**，所以文件开头的字符串/数据会被当作指令执行。

### 3.2 指令语义要点

- 操作数在内存里是 4 个独立字节，VM 用 `hexcat` 拼成地址：

  ```c
  addr = (((((b0 << 8) + b1) << 8) + b2) << 8) + b3;
  ```

  注意它用的是完整 `int` 值再移位/相加，不是简单按字节 mask。本实现的 VM 也按这个方式拼地址，否则自修改/特殊数据会与原版不一致。

- 大多数指令是“地址的地址”：先读操作数地址，再 `memory[addr]` 取值。
- `mov addr imm` 是例外：直接把 `imm` 写进 `memory[addr]`。
- `geta addr`：`memory[addr] = PC + 5`。
- `goto addr`：`PC = memory[addr]`。
- `> < =`：比较前两个地址里的值，为真则 `PC = memory[第三个操作数]`，否则 `PC += 13`。
- `not a b dest`：`dest = (a != b)`，即 3 操作数的不等比较，不是逻辑非。
- `lm a b`：`target = address_of_a - *b; memory[target] = *a; *a = 0`。
- `rm a b`：同上，但 `target = address_of_a + *b`。

注意：community VM 把这里实现成了 `target = *a ± *b`，与原版不同。本项目的 Rust VM 默认按原版语义，`--community` 切换为 community 语义。
- 未知 opcode：NOP，`PC += 1`。
- `0xff`：进入数据模式，跳过字节直到下一个 `0xff`；结束标记本身也不再执行。

### 3.3 原始 VM 的 opcode 表

`0x01..0x04, 0x11..0x13, 0x21..0x2a, 0x31..0x36`，加上数据标记 `0xff`。

**没有 `0x41..0x46`。** 原始 VM 对未知 opcode 的处理是 NOP 后 `PC += 1`，所以如果 ROM 里出现 `0x41`，它会跳过 1 字节，然后把后面的 4 字节操作数当作指令继续执行——结果通常是随机行为，而不是“位运算没有生效”这么简单。

## 4. 社区重建版新增了什么

社区编译器在原始格式上增加了：

- `[label]` 声明、`@label` 引用（4 字节大端绝对地址）。
- `#`、`--`、`/* */` 注释。
- 字符串转义 `\n \t \r \xHH`；未知转义保留字符本身（所以 `\0` 是 `'0'`）。
- `sto` 助记符（就是 `0xff`）。
- 位运算 `& | ^ ~ << >>`（opcode `0x41..0x46`）。

早期 community rebuild 还把 `not` 实现成 2 操作数逻辑非，并把 `lm/rm` 的基址从原版的“第一个操作数的地址”改成了“第一个操作数的值”。本仓库已经把 community C 实现修正为原版语义；Rust VM 也统一按原版解释。

其中 label、注释、字符串、`sto` 都只是**编译期**功能，生成给原始 VM 的 ROM 仍然只包含原始 opcode，因此完全兼容（前提是别用位运算）。

重复 label 的行为：社区编译器把 label 头插到链表，`find_addr` 从表头找，所以**最后一次定义生效**。本实现默认复刻该行为，`--strict` 才报错。

## 5. 为什么 community 位运算不能“完美降级”

核心矛盾：XASM 只有一个平坦内存，代码、数据、栈、临时变量全靠地址区分，而且没有保留机制。

假设把：

```asm
& 0x00 0x00 0x01 0x00 0x00 0x00 0x01 0x01
```

展开成一段只用原始指令的循环。至少需要：

1. **多个临时单元**：被操作数、循环计数、结果、位掩码、余数等。
2. **循环与条件跳转**：需要 label/地址，展开后所有后续地址都会变化，必须重新布局。
3. **临时单元地址不能和用户内存冲突**：用户可能把栈放在 `0xf000`、堆放在 `0xff00`，也可能读写任意地址；编译器无法知道。
4. **不能破坏用户代码/数据**：XASM 允许 `mov/copy` 自修改，允许 `goto` 跳到任意地址，甚至跳到数据中间。
5. **有符号语义**：VM 的单元是 `int`，`~`、`>>`、负数、移位计数 ≥ 32 的行为需要逐位模拟。
6. **没有可重入栈**：如果位运算出现在被递归调用的函数里，单份临时单元会被覆盖。

即使愿意承担这些，仍然无法保证“完美”：

- 临时单元可能与用户数据重叠；
- 展开代码可能改变 `geta` 的返回值、label 地址、`sto` 数据块边界；
- 原程序可能依赖具体的代码字节布局（自修改、跳入数据）；
- 一旦用户程序也使用同一块临时地址，结果不可预测。

因此，**把 community 指令降级为原始指令不是“替换一条指令”，而是引入一套新的运行时 ABI**。这超出了“兼容旧 VM”的合理范围。本实现选择：

- `--target legacy`：源码里出现 `0x41..0x46` 助记符 -> 编译错误，明确告诉用户原因和替代方案。
- `--target community`：原样生成，交给 Rust/community VM 执行。
- 不提供“自动降级”开关，避免给出“能跑”的错觉。

## 6. 兼容性矩阵

| 项目 | 原始编译器 | community 编译器 | 本实现 | 原始 VM | Rust VM |
|---|:---:|:---:|:---:|:---:|:---:|
| 23 条原始指令 | ✅ | ✅ | ✅ | ✅ | ✅ |
| `0xff` 数据块 | ✅（原始字节） | ✅（`sto`/`0xff`） | ✅ | ✅ | ✅ |
| `//` 注释 | ✅ | ✅ | ✅ | — | — |
| `#` / `--` / `/* */` | ❌ | ✅ | ✅ | — | — |
| `[label]` / `@label` | ❌ | ✅ | ✅ | 只是数据 | 只是数据 |
| 字符串转义 | ❌（原样） | ✅ | ✅（`--raw-strings` 可关） | — | — |
| 重复 label | — | 最后生效 | 最后生效 / `--strict` 报错 | — | — |
| `0x41..0x46` | ❌ | ✅ | 仅 `--target community` | ❌ | ✅ |
| 裸十六进制 `ff` | ❌ | ✅（>0xff 截断） | ✅（>0xff 报错） | — | — |

## 7. 验证证据

### 7.1 原始编译器逐字节差分

`crates/xasm-core/tests/differential.rs`：

- `tests/fixtures/temp.asm`（原始作者提供的示例）逐字节等于 `temp.disk.bin`。
- 全部 23 条原始指令 + 注释 + 字符串 + `0xff` 数据块。
- 40 组确定性随机程序。
- 运行方式：测试会定位 `original/XASM_V1/compile`，在临时目录执行，比较 `disk.bin`。

### 7.2 community 编译器逐字节差分

对 `../XASM_Community_Rebuild/examples` 中 community 编译器能成功编译的示例进行比较，包括：

- `array/array.xasm`
- `console/console.xasm`
- `input/get_line.unfinished.xasm`
- `maths/frac/frac.xasm`
- `maths/mod/mod.xasm`
- `os/os.xasm`
- `print/print.xasm`
- `string/strcpy.xasm`（空文件）

`maths/gcd/gcd.xasm` 会让 community 编译器自身卡住，`maths/pi/pi.xasm` 会让它报错，`demo/*.xasm` 是旧格式，因此差分测试会跳过这些文件。

### 7.3 原始 VM 端到端

`crates/xasm-vm/tests/old_vm.rs`：

- 用 `xasm` 编译一个 `putc 'H'` 的 ROM；
- 写入临时目录的 `disk.bin`；
- 用原始 `XASM1` 执行；
- 断言 stdout 恰好为 `H`。

另外手工验证过 community `console.xasm` 在原始 VM 与 Rust VM 上输出完全一致：

```
Hello World!
Console>
```

### 7.4 首字节/入口测试

`cargo test` 中的 `warns_when_execution_starts_with_data` 覆盖“文件以字符串开头”的情况，编译器会提示：

```
execution starts at 0x00000000 with 0x48, which is not a known instruction...
```

## 8. 已知差异 / 非目标

- 不复刻原始编译器的“无 token 边界贪心匹配”（`add0x01`）和“`0x1` 吃掉空格”等 bug；合法 token 化程序输出一致。
- 裸十六进制 `dead` 这类 >0xff 的值：community 会 `fputc` 截断，本实现报错。
- 字符串 `\0` 按 community 语义是 ASCII `'0'`，不是 NUL；需要 NUL 请用 `\x00`。
- Rust VM 的 `getc` 在 TTY 上同样用 termios 临时关闭 `ICANON|ECHO`；管道/文件不受影响。
- Rust VM 默认把 ROM 之后的内存填 `-1`（原版 EOF 行为）；community VM 是 0，可用 `--zero-fill` 切换。
- `not` 统一为 3 操作数 `dest = (a != b)`；community C 实现已修正。
- `lm/rm` 统一用 `address_of_a ± *b`；community C 实现已修正。
- Rust VM 对越界、除零、未终止数据块返回错误；原版是段错误/SIGFPE/继续扫描。
- 默认不自动插入入口跳转：原版从 0 开始，本实现保持字节布局不变；需要数据在前时请用 `0xff ... 0xff` 或把代码放前面。可选的 `--entry <label>` 会插入 14 字节跳板（`mov 0, @label; goto 0`），并让所有 label 后移 14 字节，因此只应在需要时使用。

## 9. 复现实验

```bash
# 1. 构建
cargo build --release

# 2. 原始示例逐字节比较
cargo test -p xasm-core --test differential

# 3. 原始 VM 端到端
cargo test -p xasm-vm --test old_vm

# 4. 手工替换原版编译器
cd /tmp/prog
cp /path/to/xasm .
./xasm --legacy
./XASM1
```

原始 VM 每次启动约 4–5 秒是正常的（逐字节 `fseek` 加载 1 MiB）。
