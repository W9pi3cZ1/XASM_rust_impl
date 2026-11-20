# XPL：给 XASM 用的现代一点的小语言

XPL 是一个面向 XASM 的小型高层语言。它只使用原始 XASM1 的 23 条指令和 `0xff`，所以编译出的 ROM 可以直接放进原版 `XASM1` 运行。

目标是“写起来像现代语言，跑起来还是纯 XASM”：

```xpl
// hello.xpl
print("Hello, XPL!\n");

let a = 10;
let b = 3;
print("a + b = ", a + b, "\n");

if (a > b) {
    print("a is greater\n");
} else {
    print("b is greater\n");
}

let i = 5;
while (i > 0) {
    print(i, " ");
    i = i - 1;
}
println();
```

## 快速开始

```bash
cargo build --release

# 编译
./target/release/xplc -i examples/xpl/hello.xpl -o hello.bin

# 编译并立即用 Rust VM 运行
./target/release/xplc -i examples/xpl/hello.xpl -o hello.bin --run

# 查看生成的 XASM 汇编
./target/release/xplc -i examples/xpl/hello.xpl -o hello.bin --print-asm
./target/release/xplc -i examples/xpl/hello.xpl -o hello.bin --emit-asm hello.asm
```

放进原始 VM：

```bash
mkdir -p run && cp hello.bin run/disk.bin
cd run
/path/to/XASM1
```

## 语法

### 注释

```xpl
// line comment
# another line comment
/* block comment */
```

### 变量

```xpl
let x = 10;     // 带初始化
let y;          // 默认 0
x = x + 1;      // 赋值
```

- 变量是 32 位有符号整数。
- `{}` 产生块作用域，支持遮蔽（shadowing）。
- 同一个作用域内重复 `let` 会报错。
- 初始化表达式在绑定新名字之前求值，所以：

```xpl
let x = 10;
{
    let x = x + 1;  // 右边的 x 是外层的 10
    print(x);       // 11
}
print(x);           // 10
```

### 字面量

```xpl
42
0x2a
0xffffffff   // 按 32 位位模式解释，等于 -1
true
false
"hello\n"
```

- 十进制最大 `2147483647`；更大的负数请写 `-5` 或 `0x80000000`。
- 十六进制最多 32 位，超过 `0xffffffff` 报错。
- 字符串转义：`\n`、`\t`、`\r`、`\0`、`\\`、`\"`、`\'`、`\xHH`。
- 字符串只能作为 `print`/`println` 的参数，不是一等值。

### 运算符

优先级从低到高：

| 运算符 | 结合性 | 说明 |
|---|---|---|
| `\|\|` | 左 | 逻辑或，短路 |
| `&&` | 左 | 逻辑与，短路 |
| `==` `!=` `<` `<=` `>` `>=` | 左 | 比较，结果是 0/1 |
| `+` `-` | 左 | 加减 |
| `*` `/` `%` | 左 | 乘、除、取余 |
| `-` `+` `!` | 右 | 一元负号、正号、逻辑非 |

- `/` 是 C 风格截断除法；`%` 是 C 风格余数，例如 `-7 / 2 == -3`、`-7 % 2 == -1`。
- 32 位溢出按补码回绕。
- `&&` 和 `||` 会短路，右侧的 `read_int()` 等副作用不会执行。
- 条件（`if`/`while`）把任何非 0 值视为真，包括负数。

### 控制流

```xpl
if (cond) {
    ...
} else if (other) {
    ...
} else {
    ...
}

while (cond) {
    ...
}
```

- 支持 `else if`。
- 目前没有 `for`、`break`、`continue`。

### 简单 I/O

```xpl
print("text");          // 字符串
print(x);               // 十进制整数
print("x = ", x, "\n"); // 多个参数
println();              // 只换行
println("done");        // 字符串 + 换行
putchar(65);            // 输出一个字符，65 = 'A'

let n = read_int();     // 读十进制整数；EOF 时保持 0
let c = read_char();    // 读一个字节，不跳过空白
let h = read_hex();     // 读十六进制整数

puthex(expr);           // 输出小写十六进制（无前导 0）
puthex_at(addr);        // 直接输出 memory[addr] 的十六进制
let b = peek(addr);     // 读任意内存 cell（自修改 lm + restore）
poke(addr, value);      // 写任意内存 cell
```

`peek`/`puthex_at`/`poke` 让 XPL 可以写内存查看/编辑工具。`examples/xpl/memedit.xpl` 就是一个交互式内存编辑器：

```bash
./target/release/xplc -i examples/xpl/memedit.xpl -o memedit.bin --run
```

在 TTY 上它使用 `getc` 的原始模式，直接响应单键：`h/l` 移动光标，`j/k` 移动一行，`0-9a-f` 编辑当前字节（输入两个十六进制数字），`q` 退出。显示使用 alternate screen（`\x1b[?1049h/l`）、清屏/反显，并在退出时恢复光标和 shell 画面；每个字节固定打印两位十六进制（`peek` 取高/低 nibble），所以列是对齐的。不是 TTY 时会按普通字节流读取。

`puthex_at` 使用 `puth` 的指针参数：把运行时地址写进 `@__ptr`，再 `puth @__ptr`，因此不需要间接读或自修改代码。编辑器为了补零使用 `peek`，它用自修改的 `lm` 读值并立即 restore（XASM 没有间接读）。`poke` 使用原版 `lm` 的 `target = address_of_a - *b` 语义。community rebuild 里这处不一致已经修正，因此 `xasm-run` 和修正后的 community VM 都可以执行。

> 说明：`memedit.xpl` 在 Rust VM 和原始 `XASM1` 上都验证通过。原版 VM 有一个 dispatch fallthrough bug（`copy` 的最后一个操作数字节可能被当作 opcode），XPL 通过“条件跳转 NOP 填充 + 变量/临时变量 256 字节对齐”规避。代价是 ROM 变大（约 44 KB），但仍远低于 1 MiB。详细分析见 [xasm1-dispatch-bug.md](xasm1-dispatch-bug.md)。

- `read_int()` 按 `scanf("%d")` 风格跳过前导空白。
- `read_char()` 按 `getch` 风格直接读下一个字节，不跳过换行。`read_int()` 之后通常还剩一个换行，需要自己处理。

## 编译模型

XPL 没有寄存器、栈指针或函数调用；它使用一段很简单的平坦内存 ABI。

ROM 布局：

```text
0x00000000  代码
             ...
             exit
             [__zero] [__one] [__newline] [__jmp] [__ptr] [__cnt]
             [v0] [v1] ...       变量，每个 1 个 cell
             [t0] [t1] ...       表达式临时 cell
             [s0] ...            字符串字节
```

- 每个变量/临时变量是**一个** 32 位 cell；ROM 里的占位字节只是 0，运行时用 `mov`/`copy` 写入。
- 数据段放在 `exit` 之后，VM 不会执行它。
- `@__jmp` 是所有条件/无条件跳转共用的“跳转目标 cell”。每次跳转前用 `mov @__jmp @label` 设置。
- `@__ptr` / `@__cnt` 用于字符串打印循环。
- 字符串按长度循环打印：

```asm
mov @__ptr @str
mov @__cnt <len>
[strloop]
putc @__ptr
add @__ptr @__one
sub @__cnt @__one
mov @__jmp @strloop
> @__cnt @__zero @__jmp
```

- 表达式写入一个目标 cell；二元运算先求左值到目标 cell，再求右值到临时 cell，然后 `add/sub/xadd/xsub`。
- 取余用 `a - (a / b) * b` 实现，因为 XASM 没有取余指令。
- 比较生成 0/1：条件跳转到 `true` 分支，再汇合。
- `print(expr)` 先把临时 cell 的地址放进 `@__ptr`，再 `putn @__ptr`。
- 所有临时 cell 都是编译期分配的，运行时不会冲突。

### 一个最小的生成例子

```xpl
let x = 3;
if (x > 2) { print("yes"); } else { print("no"); }
```

生成的 XASM 汇编（省略数据段）：

```asm
mov @t0 0x00 0x00 0x00 0x03
copy @v0 @t0
copy @t1 @v0
mov @t2 0x00 0x00 0x00 0x02
mov @__jmp @cmp_true_0
> @t1 @t2 @__jmp
mov @t1 0x00 0x00 0x00 0x00
mov @__jmp @cmp_end_1
goto @__jmp
[cmp_true_0]
mov @t1 0x00 0x00 0x00 0x01
[cmp_end_1]
mov @__jmp @else_2
= @t1 @__zero @__jmp
mov @__ptr @s0
...
```

## 与原版 VM 的关系

`xplc` 只生成：

- `0x01..0x04` 算术
- `0x11..0x13` 逻辑
- `0x21..0x2a` 控制/内存
- `0x31..0x36` I/O
- `0xff` 数据块标记

不会生成 community 的 `0x41..0x46`。因此 XPL 输出的 ROM 不需要 community VM，也不需要任何兼容层。

测试 `crates/xasm-lang/tests/old_vm.rs` 会把 XPL 编译出的 ROM 放进原版 `XASM1` 运行并比对输出。

## CLI

```text
xplc -i <FILE> [OPTIONS]

-i, --input <FILE>      XPL 源文件
-o, --output <FILE>     ROM 输出（默认 out.bin）
    --emit-asm <FILE>   写生成的 XASM 汇编
    --print-asm         把生成的汇编打印到 stdout
    --run               编译后用 Rust XASM VM 运行
    --max-addr <ADDR>   VM 最高地址（默认 0xffffe）
    --stats             输出 ROM/代码生成统计
-q, --quiet
-v, --verbose
```

## 示例

见 `examples/xpl/`：

- `hello.xpl`
- `countdown.xpl`
- `fib.xpl`
- `fizzbuzz.xpl`
- `input_sum.xpl`
- `primes.xpl`
- `memedit.xpl`（交互式内存编辑器）

## 当前限制 / 可以继续做的方向

- 没有用户函数、递归、调用栈。
- 没有数组、指针、结构体。
- 字符串不是一等值，只能打印。
- 没有 `for`、`break`、`continue`。
- `poke` 使用原版 `lm` 语义；`puthex_at` 只读显示，不修改内存。
- `peek` 在原始 VM 上用“自修改 `lm` + 立即 restore”实现，`lm` 的第二个操作数放在 256 字节对齐的 cell 上，所以不再受 `copy` 那个“最后一个操作数字节被当成 opcode”的 bug 影响。
- 没有真正的类型系统；所有值都是 i32，布尔值是 0/1。
- 除数为字面量 0 会在编译期报错；运行时除 0 会触发 VM 错误（原版是 SIGFPE）。
- 没有运行时越界/溢出检查。

这些限制是为了保证生成代码仍然只是“纯 XASM”，而不是引入一个新的运行时。后续如果要加函数，需要先设计一套软件栈和调用约定。
