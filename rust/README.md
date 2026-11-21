# XASM Rust 实现

用 Rust 重写的 XASM 编译器与虚拟机。默认目标是最初的 **XASM1（Linux amd64 二进制）**，同时兼容社区重建版（community rebuild）的源码语法。

本目录是一个独立的 Rust 项目，也可以单独作为 git 仓库；当前工作区根目录把它作为 `rust/` 子目录提交。手写的 community 实现是根目录的 git submodule：`../XASM_Community_Rebuild`。

- `xasm`：编译器。输出与原始编译器相同的 `opcode + 4 字节大端地址` ROM 格式。
- `xasm-run`：虚拟机。默认复刻原始 `XASM1` 语义，并实现 community 的 6 条位运算指令，便于开发调试。

> 结论先说：**共享指令集可以做到字节级兼容**；community 独有的 `& | ^ ~ << >>`（opcode `0x41..0x46`）**不能**在原始 VM 上执行。本项目不会假装能用“简单替换”把它们降级过去，而是在 `--target legacy` 下明确报错，在 `--target community` 下原样生成。原因见 [docs/compatibility.md](docs/compatibility.md)。

## 快速开始

构建需要 Rust 1.85+；直接依赖是 `clap 4` 和 Unix 下的 `libc`（`Cargo.lock` 已提交，便于复现）。网络受限时可使用代理或 `cargo build --offline`（依赖已缓存时）。

```bash
cargo build --release

# 方式一：替换原始 IDE 的工作流（固定读 ./temp.asm，写 ./disk.bin）
cd /path/to/program
/path/to/xasm --legacy

# 也可以零参数直接运行：目录里有 temp.asm 时自动进入 legacy 模式
/path/to/xasm

# 方式二：现代 CLI
xasm -i program.xasm -o disk.bin
xasm-run -i disk.bin

# 零参数运行：目录里有 disk.bin 时自动读取
xasm-run

# 使用 community 扩展指令（只能在 community/Rust VM 上跑）
xasm -i program.xasm -o program.bin --target community
xasm-run -i program.bin
```

仓库自带的示例：

```bash
cargo run -q -p xasm-cli -- -i examples/hello.xasm -o /tmp/hello.bin
cargo run -q -p xasm-vm  -- -i /tmp/hello.bin --stats

cargo run -q -p xasm-cli -- -i examples/community_bits.xasm -o /tmp/bits.bin --target community
cargo run -q -p xasm-vm  -- -i /tmp/bits.bin

cargo run -q -p xasm-cli -- -i examples/data_first.xasm -o /tmp/data.bin --entry main
```

原始 VM 用法不变：把 `disk.bin` 放在 `XASM1` 同目录运行。原始 `XASM1` 每次启动会逐字节 `fseek+fgetc` 读取约 1 MiB 内存，因此大约需要 4–5 秒才开始执行，这是原版行为，不是本项目引入的。

## 兼容性约定

| 能力 | 原始 XASM1 VM | Rust VM (`xasm-run`) |
|---|:---:|:---:|
| 23 条原始指令（`0x01..0x2a`, `0x31..0x36`） | ✅ | ✅ |
| `0xff` / `sto` 数据块 | ✅ | ✅ |
| `[label]` / `@label`（编译期展开为 4 字节地址） | ✅（只是数据） | ✅ |
| `//`、`#`、`--`、`/* */` 注释 | ✅（只是被编译器忽略） | ✅ |
| `& \| ^ ~ << >>`（community 位运算） | ❌ | ✅ |

编译器默认 `--target legacy`：

- 23 条原始指令 + `sto`：正常生成。
- 源文件里出现 community 位运算助记符：编译失败，给出精确行号和替代建议。
- 仍想强行生成 `0x41..0x46`（例如给 Rust/community VM 测试）：`--allow-community`，会给出警告。
- `--target community`：允许并原样生成 `0x41..0x46`。

### 为什么不把 community 指令降级成原始指令？

XASM 只有一个平坦的、按地址访问的内存；没有寄存器、栈指针、可执行段/数据段，也没有“保留临时单元”的机制。要模拟一条 `a = a & b`，至少需要：

1. 若干临时内存单元；
2. 循环/分支与计数器；
3. 保证这些临时单元不与用户程序的数据、代码、`geta` 返回地址冲突；
4. 处理有符号 32 位、移位计数、`~`、`<<`、`>>` 的边界语义；
5. 处理程序可能跳进数据、代码可能被 `mov/copy` 自修改的情况。

这些条件在“一个地址区分一切”的 XASM 里无法同时保证。简单替换会改变指令长度和所有后续地址，必须做全局重定位和运行时状态模拟；那已经是一个新的 ABI，而不是“兼容”。所以本项目选择**显式的目标边界**，而不是不可靠的自动降级。

## 编译器

```text
xasm [OPTIONS] <INPUT>
xasm --legacy

-i, --input <FILE>       输入源文件
-o, --output <FILE>      输出 ROM（默认 out.bin；--legacy 下默认 disk.bin）
    --legacy             使用原始固定路径 temp.asm -> disk.bin
    --target <T>         legacy（默认）或 community
    --community          等价于 --target community
    --allow-community    legacy 目标下仍生成 0x41..0x46（危险）
    --raw-strings        不处理字符串里的 \n \t \xHH 等转义（原版行为）
    --entry <LABEL>      在 ROM 开头插入 14 字节跳板，跳到 LABEL
    --max-addr <ADDR>    VM 可访问的最高地址（默认 0xffffe）
    --verify <LEVEL>     off / warn（默认）/ error
    --strict             --verify error，并把重复 label 视为错误（不做控制流分析）
    --listing <FILE>     输出反汇编列表
    --stats              输出 ROM 统计
    --check              只编译和校验，不写文件
-q, --quiet              安静模式
-v, --verbose            输出进度
-h, --help
-V, --version
```

### 入口地址提醒

原始 VM 的程序计数器从 `0x00000000` 开始。**如果源文件开头是字符串/数据，它们会被当成指令执行。** 编译器在 `--verify warn`（默认）下会检测首字节不是已知指令的情况并警告。两种正确写法：

```asm
-- 写法 A：代码在前，数据在后
[main]
mov ...
exit
[msg] "Hello"

-- 写法 B：数据用 0xff ... 0xff 包起来（原版常用）
0xff
"Hello"
0xff
[main]
mov ...
exit
```

如果不想改源码，也可以用可选的入口跳板：

```bash
xasm -i data_first.xasm -o disk.bin --entry main
```

它会在 ROM 开头插入 14 字节：

```asm
mov 0x00000000 @main   ; 把入口地址写到 cell 0
goto 0x00000000        ; 通过 cell 0 跳转
```

所有 label 地址会相应后移 14 字节。该选项会改变 ROM 布局，默认不启用，因此默认输出仍与原始/community 编译器逐字节可比。

## 语言参考

XASM 是字节流汇编器：每个 token 产生 0、1 或 4 个字节。

### 数字与数据

- `0xHH`：一个字节；也接受 `0x1`、`0x0001`，但值必须 ≤ `0xff`。
- 裸十六进制字节（community 兼容）：`ff`、`10`，同样必须 ≤ `0xff`。
- 操作数是 4 个独立字节，按大端序拼成 32 位地址：

```asm
mov 0x00 0x00 0x01 0x00 0x00 0x00 0x00 0x48
```

### 字符串

```asm
"Hello\n"
'also a string'
```

- 默认按 community 规则处理转义：`\n`、`\t`、`\r`、`\\`、`\"`、`\'`、`\xHH`。
- 未知转义保留字符本身，因此 `\0` 是 ASCII `'0'`（community 编译器行为）。
- `--raw-strings`：反斜杠按普通字节处理，与原版编译器一致。
- 字符串不会自动添加结尾 `\0`。

### 注释

```asm
// line
# line
-- line
/* block */
```

### Label 与引用

```asm
[msg] "Hi"
[ptr] 0x00 0x00 0x00 0x00

mov @ptr @msg
putc @ptr
```

- `[name]` 记录当前字节偏移。
- `@name` 展开为 4 字节大端绝对地址。
- 重复 label 采用**最后一次定义**（与 community 编译器一致）；`--strict` 下报错。

### 指令表

| 助记符 | Hex | 操作数 | 说明 |
|---|---|---|---|
| `add` | 0x01 | 2 | 加法 |
| `sub` | 0x02 | 2 | 减法 |
| `xadd` | 0x03 | 2 | 乘法 |
| `xsub` | 0x04 | 2 | 除法 |
| `and` | 0x11 | 3 | 逻辑与，结果写第 3 个地址 |
| `or` | 0x12 | 3 | 逻辑或，结果写第 3 个地址 |
| `not` | 0x13 | 3（原版）/ 2（community） | 原版 `dest = (a != b)`；community 是逻辑非 |
| `mov` | 0x21 | 2 | 把立即数写入地址 |
| `copy` | 0x22 | 2 | 复制 |
| `goto` | 0x23 | 1 | 间接跳转 |
| `geta` | 0x24 | 1 | 保存下一条指令地址（pc+5） |
| `>` | 0x25 | 3 | 大于则跳转 |
| `<` | 0x26 | 3 | 小于则跳转 |
| `=` | 0x27 | 3 | 等于则跳转 |
| `lm` | 0x28 | 2 | “左移”内存单元（见下） |
| `rm` | 0x29 | 2 | “右移”内存单元（见下） |
| `exit` | 0x2a | 0 | 停止 |
| `putc` | 0x31 | 1 | 输出字符 |
| `putn` | 0x32 | 1 | 输出十进制 |
| `puth` | 0x33 | 1 | 输出十六进制 |
| `getc` | 0x34 | 1 | 读一个字符 |
| `getn` | 0x35 | 1 | 读十进制整数 |
| `geth` | 0x36 | 1 | 读十六进制整数 |
| `sto` | 0xff | 0 | 数据块标记，跳过直到下一个 0xff |
| `&` | 0x41 | 2 | 按位与（community） |
| `\|` | 0x42 | 2 | 按位或（community） |
| `^` | 0x43 | 2 | 按位异或（community） |
| `~` | 0x44 | 1 | 按位取反（community） |
| `<<` | 0x45 | 2 | 左移（community） |
| `>>` | 0x46 | 2 | 右移（community） |

`lm` / `rm` 不是位移。原版语义是：

- **原版 XASM1**：`lm a b` 计算 `target = address_of_a - *b`，把 `*a` 写到 `memory[target]`，再把 `*a` 清零；`rm` 用 `+`。
- 早期 community rebuild 的实现把基址写成了 `*a`；本仓库已经把它修正为原版语义。Rust VM 统一使用原版语义。

XPL 的 `poke` 使用原版 `lm` 公式。

## 虚拟机

```text
xasm-run -i <ROM> [OPTIONS]

-i, --input <ROM>
-m, --memory <N>       最高地址（默认 0xffffe）
-c, --clock <HZ>       指令速率（0 = 不限速）
    --strict           未知 opcode 报错（默认按原版视为 NOP）
    --community        community VM 行为（0x00 停机 + 内存 0 填充 + 未知 opcode 报错）
    --zero-halts       0x00 视为停机
    --zero-fill        ROM 之后内存填 0（community 行为）
    --trace            逐条打印到 stderr
    --max-steps <N>    指令数上限
    --dump-memory      运行后 dump 内存
    --dump-start/--dump-end
    --stats
```

与原始 VM 的差异（仅开发便利，不影响编译器 ROM）：

- 默认 ROM 之后的内存单元填 `-1`：原版用 `fgetc` 填满 `0xffffe` 个单元，EOF 后得到 `-1`。`--zero-fill` 或 `--community` 改为 0。
- `getc` 在 TTY 上会像原版 `getch()` 一样临时关闭 `ICANON|ECHO`，读一个字节后恢复；管道/文件不受影响。
- 除零、越界访问返回错误，原版是 SIGFPE/未定义行为。
- 未终止的 `0xff` 数据块返回错误，原版会继续扫描到内存末尾。
- 默认未知 opcode = NOP；`--strict` 报错。

## XPL 高层语言

除了直接写 XASM，本项目还带一个面向 XASM 的小型高层语言 **XPL**。它有变量、作用域、函数、数组、表达式、`if/else`、`while`、字符串/数字输出和 `read_int`/`read_char`，并编译成只含原始 XASM1 指令的 ROM。

```xpl
// examples/xpl/fizzbuzz.xpl
let i = 1;
while (i <= 100) {
    if (i % 15 == 0) {
        print("FizzBuzz\n");
    } else if (i % 3 == 0) {
        print("Fizz\n");
    } else if (i % 5 == 0) {
        print("Buzz\n");
    } else {
        print(i, "\n");
    }
    i = i + 1;
}
```

```bash
./target/release/xplc -i examples/xpl/hello.xpl -o hello.bin
./target/release/xplc -i examples/xpl/fizzbuzz.xpl -o fizzbuzz.bin --run
./target/release/xplc -i examples/xpl/hello.xpl -o hello.bin --print-asm

# 交互式内存编辑器（终端里用 h/l/j/k、0-9a-f、q）
./target/release/xplc -i examples/xpl/memedit.xpl -o memedit.bin --run

# 整数 spigot 算法不断计算、输出 pi（函数 + 数组）
./target/release/xplc -i examples/xpl/pi.xpl -o pi.bin --run

# 原始终端 readline（缓冲区地址用 i32 传递，带回显/退格）
./target/release/xplc -i examples/xpl/readline.xpl -o readline.bin --run
```

XPL 还提供 `peek(addr)`、`poke(addr, value)`、`puthex_at(addr)`、`puthex(expr)`、`read_hex()`，所以可以直接写内存查看/编辑工具。`examples/xpl/memedit.xpl` 是一个可交互的内存编辑器：alternate screen + 清屏、16 字节一行、两位补零十六进制、光标反显，支持光标移动、按字节编辑和退出。它依赖 `getc` 的原始终端模式，`xasm-run` 已经实现；Rust VM 和原始 `XASM1` 上都验证过。

XPL 的语法、语义、生成模型和限制见 [docs/xpl.md](docs/xpl.md)。它没有引入新的运行时：变量/临时变量都是数据段里的 cell，字符串用长度循环打印，函数使用静态参数/返回槽，数组是连续 cell，所有跳转共用一个 `@__jmp` cell。

原版 `XASM1` 还存在一个 dispatch fallthrough bug：`copy`/`geta`/`>`/`<` 之后可能把操作数字节当成 opcode，导致布局敏感崩溃；XPL 通过 NOP 填充和 256 字节数据对齐规避，详见 [docs/xasm1-dispatch-bug.md](docs/xasm1-dispatch-bug.md)。最小复现实例（`temp.asm` + 原版 `compile` 生成的 `disk.bin`）在 [repro/](repro/)。

## 验证

```bash
cargo test --workspace
```

测试包括：

1. **原始编译器差分**：`temp.asm`、全部原始指令、40 组随机程序与 `original/XASM_V1/compile` 输出逐字节一致。
2. **community 编译器差分**：对 community 仓库中能正常编译的示例逐字节比较。
3. **原始 VM 端到端**：Rust 编译出的 ROM 放进原版 `XASM1` 运行并比对 stdout。
   `--verify` 是线性扫描，只用于首字节/空 ROM 提示与统计；XASM 允许跳入数据，所以它不会对“未知字节/截断”做控制流断言。
4. **VM 单元测试**：算术、逻辑、跳转、`geta`、`sto`、`lm/rm`、I/O、community 位运算。
5. **CLI 测试**：`--legacy`、`--target community`、`--listing`、`--check`、`--raw-strings`。
6. **XPL 测试**：算术/比较/短路、作用域、循环、字符串、输入输出、错误诊断，以及把 XPL ROM 放进原版 `XASM1` 执行。

## 目录结构

```text
crates/xasm-core/   词法、语法、label、布局、ROM 生成、legacy 校验
crates/xasm-cli/    xasm 命令行
crates/xasm-vm/     VM 库 + xasm-run 命令行
crates/xasm-lang/   XPL 高层语言编译器 xplc
docs/               兼容性说明、逆向记录、XPL 文档、XASM1 dispatch bug 分析
tests/fixtures/     原始编译器基准文件
repro/              原版 XASM1 dispatch bug 的最小复现（temp.asm + disk.bin）
```

## License

MIT
