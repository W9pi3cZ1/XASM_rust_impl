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


### 函数

```xpl
fn add(a, b) {
    return a + b;
}

fn twice(x) {
    return add(x, x);
}

print(twice(3), "\n");   // 6
```

- 函数只能在顶层声明，参数和返回值都是 i32。
- 用 `return expr;` 返回；没有 `return` 或 `return;` 时返回 0。
- 函数可以调用其他函数，也可以访问顶层变量/数组。
- **不支持递归**，也不支持把函数当值传递；直接递归会在编译期报错。
- 参数和局部变量是静态槽位，不支持重入/递归调用；普通嵌套调用没有问题。
- 函数体在顶层 `exit` 之后生成，通过 `goto` 进入、通过返回槽回到调用点。

### 数组

```xpl
let a[10];       // 10 个 i32 cell，初始为 0

let i = 0;
while (i < 10) {
    a[i] = i * i;
    i = i + 1;
}
print(a[3], "\n");   // 9
```

- `let a[N];` 声明长度为 N 的数组，N 必须是正整数。
- 读写作 `a[i]`，索引在运行时计算；底层用 `peek`/`poke` 访问连续内存。
- 数组有静态存储，加载时清零；不支持初始化列表，也不会在重复进入作用域时自动清空。
- 没有越界检查；越界会读写其他内存。
- 数组不是一等值：不能赋值给变量，也不能作为函数参数/返回值；函数可以直接访问全局数组。
- 数组元素和变量一样是 i32。


### 字符串、值传递和 `readline`

XPL 目前**没有字符串变量**，也没有字符串参数/返回值：

```xpl
let s = "hello";     // 报错：字符串只能作为 print/println 参数
fn f(s) { print(s); }
f("hello");          // 报错：字符串不能作为函数实参
```

字符串字面量只会被编译成 ROM 里的只读字节数组，直接交给 `print`/`println`。函数参数、返回值、变量和数组元素全部是 i32。

所以：

- **标量 i32**：值传递。`f(x)` 里改形参不会改调用方的 `x`。
- **数组**：不是一等值，不能直接传给函数，也不能 `return` 一个数组。
- **地址 i32**：地址本身是值传递，但通过 `peek`/`poke` 读写它指向的内存，效果类似“引用传递”。这是目前传缓冲区的方式。
- **字符串字面量**：只读 ROM 数据，没有值/引用传递一说。

一个可用的 `readline` 需要“缓冲区地址 + 容量”，返回实际长度。完整示例在 [../examples/xpl/readline.xpl](../examples/xpl/readline.xpl)：

```xpl
let base = 0x80000;   // 一块可写内存
let cap = 128;

fn readline(addr, max) {
    let n = 0;
    let done = 0;
    while (done == 0) {
        let c = read_char();
        if (c == -1) {                 // EOF
            done = 1;
        } else if (c == 10 || c == 13) {
            done = 1;
            println();
        } else if (c == 8 || c == 127) { // backspace
            if (n > 0) {
                n = n - 1;
                putchar(8);
                putchar(32);
                putchar(8);
            }
        } else if (c >= 32 && c < 127) {
            if (n < max) {
                poke(addr + n, c);
                n = n + 1;
                putchar(c);            // raw mode 下没有自动回显
            }
        }
    }
    return n;
}

fn print_line(addr, len) {
    let i = 0;
    while (i < len) {
        putchar(peek(addr + i));
        i = i + 1;
    }
    println();
}

print("> ");
let n = readline(base, cap);
print_line(base, n);
```

要点：

- `read_char()` 在 `xasm-run`/原版 `getc` 下是原始单字节读取；TTY 上不回显，所以 `readline` 自己 `putchar(c)`。
- 回车按 `10`（LF）或 `13`（CR）结束；raw TTY 通常经 `ICRNL` 得到 LF。
- 退格处理 `8`/`127`，先 `n--`，再输出 `\b \b` 把屏幕上的字符擦掉。
- 只接受可打印 ASCII（32..126）。UTF-8 字节 >=128 会被忽略；方向键等 ANSI 序列也没有解析，真正的行编辑器需要额外状态机。
- 返回的是长度，不是字符串；调用方知道缓冲区地址，用 `peek` 打印或处理。

如果只有一个全局缓冲区，也可以让函数直接访问全局数组，省掉 `addr` 参数：

```xpl
let line[128];
fn readline_global(max) {
    ...
    line[n] = c;
    ...
}
```

但这样函数只能操作这一个数组；要操作多个缓冲区，就传地址（或以后加 `&array` / 数组参数）。

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

XPL 没有寄存器，也没有硬件栈；它使用一段很简单的平坦内存 ABI。函数使用静态参数/返回槽，数组是连续的 i32 cell，索引通过 `peek`/`poke` 访问。

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
- `pi.xpl`（整数 spigot 算法 + 数组/函数，不断输出 pi）
- `readline.xpl`（原始终端下带回显/退格的 readline，缓冲区地址用 i32 传递）
- `memedit.xpl`（交互式内存编辑器）

## 当前限制 / 可以继续做的方向

- 没有递归/调用栈；函数参数和局部变量是静态槽位。
- 没有指针、结构体、数组初始化列表；数组没有越界检查。
- 数组和函数都使用静态存储，不支持把数组/函数当一等值。
- 字符串不是一等值，不能赋给变量、传参或返回；只能作为 `print`/`println` 参数。
- 没有 `for`、`break`、`continue`。
- `poke` 使用原版 `lm` 语义；`puthex_at` 只读显示，不修改内存。
- `peek` 在原始 VM 上用“自修改 `lm` + 立即 restore”实现，`lm` 的第二个操作数放在 256 字节对齐的 cell 上，所以不再受 `copy` 那个“最后一个操作数字节被当成 opcode”的 bug 影响。
- 没有真正的类型系统；所有值都是 i32，布尔值是 0/1。
- 除数为字面量 0 会在编译期报错；运行时除 0 会触发 VM 错误（原版是 SIGFPE）。
- 没有运行时越界/溢出检查。

这些限制是为了保证生成代码仍然只是“纯 XASM”，而不是引入一个新的运行时。函数使用静态参数/返回槽和非递归调用约定；如果需要递归，需要再加一套软件栈。
