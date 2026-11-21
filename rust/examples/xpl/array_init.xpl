// Compile-time array initializers.
//
// Values are encoded directly into the ROM as bytes; at load time XASM cells
// 0x00..0xff become i32 values of the same value.  Strings are UTF-8 bytes.

fn print_bytes(addr, len) {
    let i = 0;
    while (i < len) {
        putchar(peek(addr + i));
        i = i + 1;
    }
}

let a[5] = [10, 20, 30, 40, 50];
let s[5] = "hello";
let utf8[3] = "中";
let pad[4] = [255, 0, 1];   // missing tail is zero-filled

print(a[2], "\n");          // 30
print_bytes(&s, 5);         // hello
println();
print_bytes(&utf8, 3);      // 中
println();

puthex(pad[0]);
print(" ", pad[1], " ", pad[2], " ", pad[3], "\n");
