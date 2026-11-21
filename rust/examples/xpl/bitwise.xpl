// Bitwise operations are lowered to arithmetic loops, so this runs on the
// original XASM1 (no 0x41..0x46 community opcodes).

let a = 0x12345678;
let b = 0x0f0f0f0f;

print("a & b = ");
puthex(a & b);
println();

print("a | b = ");
puthex(a | b);
println();

print("a ^ b = ");
puthex(a ^ b);
println();

print("~a    = ");
puthex(~a);
println();

print("a << 4 = ");
puthex(a << 4);
println();

print("a >> 4 = ");
puthex(a >> 4);
println();

let mask = 0xff;
print("-1 & mask = ", -1 & mask, "\n");
