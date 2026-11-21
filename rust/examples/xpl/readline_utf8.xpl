// UTF-8-aware readline for a raw TTY.
//
// The buffer stores UTF-8 bytes, so `n` is a byte count.  Backspace removes a
// whole codepoint and erases its display width (2 columns for CJK/emoji,
// 0 for common combining marks).  Line wrapping is not tracked.

let line[256];
let cap = 256;

fn is_cont(b) {
    return b >= 0x80 && b < 0xc0;
}

fn utf8_width(cp) {
    if (cp >= 0x0300 && cp <= 0x036f) { return 0; }
    if (cp >= 0x1ab0 && cp <= 0x1aff) { return 0; }
    if (cp >= 0x1dc0 && cp <= 0x1dff) { return 0; }
    if (cp >= 0x20d0 && cp <= 0x20ff) { return 0; }
    if (cp >= 0xfe20 && cp <= 0xfe2f) { return 0; }

    if (cp >= 0x1100 && cp <= 0x115f) { return 2; }
    if (cp >= 0x2e80 && cp <= 0xa4cf) { return 2; }
    if (cp >= 0xac00 && cp <= 0xd7a3) { return 2; }
    if (cp >= 0xf900 && cp <= 0xfaff) { return 2; }
    if (cp >= 0xfe10 && cp <= 0xfe19) { return 2; }
    if (cp >= 0xfe30 && cp <= 0xfe6f) { return 2; }
    if (cp >= 0xff00 && cp <= 0xff60) { return 2; }
    if (cp >= 0xffe0 && cp <= 0xffe6) { return 2; }
    if (cp >= 0x1f300 && cp <= 0x1f64f) { return 2; }
    if (cp >= 0x1f900 && cp <= 0x1f9ff) { return 2; }
    if (cp >= 0x20000 && cp <= 0x3fffd) { return 2; }

    return 1;
}

fn width_at(addr, start) {
    let b0 = peek(addr + start);
    if (b0 < 0x80) {
        return 1;
    }
    if (b0 >= 0xc0 && b0 < 0xe0) {
        let b1 = peek(addr + start + 1);
        return utf8_width((b0 - 0xc0) * 64 + (b1 - 0x80));
    }
    if (b0 >= 0xe0 && b0 < 0xf0) {
        let b1 = peek(addr + start + 1);
        let b2 = peek(addr + start + 2);
        return utf8_width((b0 - 0xe0) * 4096 + (b1 - 0x80) * 64 + (b2 - 0x80));
    }
    if (b0 >= 0xf0 && b0 < 0xf8) {
        let b1 = peek(addr + start + 1);
        let b2 = peek(addr + start + 2);
        let b3 = peek(addr + start + 3);
        return utf8_width(
            (b0 - 0xf0) * 262144 + (b1 - 0x80) * 4096 + (b2 - 0x80) * 64 + (b3 - 0x80)
        );
    }
    return 1;
}

fn erase(width) {
    let k = 0;
    while (k < width) {
        putchar(8);
        k = k + 1;
    }
    k = 0;
    while (k < width) {
        putchar(32);
        k = k + 1;
    }
    k = 0;
    while (k < width) {
        putchar(8);
        k = k + 1;
    }
}

fn readline(addr, max) {
    let n = 0;
    let done = 0;

    while (done == 0) {
        let b0 = read_char();

        if (b0 == -1) {
            done = 1;
        } else if (b0 == 10 || b0 == 13) {
            done = 1;
            println();
        } else if (b0 == 8 || b0 == 127) {
            if (n > 0) {
                let start = n - 1;
                while (start > 0 && is_cont(peek(addr + start))) {
                    start = start - 1;
                }
                let w = width_at(addr, start);
                n = start;
                erase(w);
            }
        } else if (b0 >= 32 && b0 < 127) {
            if (n + 1 <= max) {
                poke(addr + n, b0);
                n = n + 1;
                putchar(b0);
            }
        } else if (b0 >= 0xc2 && b0 <= 0xf4) {
            let need = 0;
            if (b0 < 0xe0) {
                need = 1;
            } else if (b0 < 0xf0) {
                need = 2;
            } else {
                need = 3;
            }

            let b1 = 0;
            let b2 = 0;
            let b3 = 0;
            let valid = 1;

            if (need >= 1) {
                b1 = read_char();
                if (!is_cont(b1)) { valid = 0; }
            }
            if (need >= 2) {
                b2 = read_char();
                if (!is_cont(b2)) { valid = 0; }
            }
            if (need >= 3) {
                b3 = read_char();
                if (!is_cont(b3)) { valid = 0; }
            }

            if (valid == 1) {
                let cp = 0;
                if (need == 1) {
                    cp = (b0 - 0xc0) * 64 + (b1 - 0x80);
                } else if (need == 2) {
                    cp = (b0 - 0xe0) * 4096 + (b1 - 0x80) * 64 + (b2 - 0x80);
                } else {
                    cp = (b0 - 0xf0) * 262144 + (b1 - 0x80) * 4096
                        + (b2 - 0x80) * 64 + (b3 - 0x80);
                }

                if (need == 2 && cp < 0x800) { valid = 0; }
                if (need == 3 && cp < 0x10000) { valid = 0; }
                if (cp >= 0xd800 && cp <= 0xdfff) { valid = 0; }
                if (cp > 0x10ffff) { valid = 0; }
            }

            if (valid == 1) {
                if (n + need + 1 <= max) {
                    poke(addr + n, b0);
                    if (need >= 1) { poke(addr + n + 1, b1); }
                    if (need >= 2) { poke(addr + n + 2, b2); }
                    if (need >= 3) { poke(addr + n + 3, b3); }
                    n = n + need + 1;

                    putchar(b0);
                    if (need >= 1) { putchar(b1); }
                    if (need >= 2) { putchar(b2); }
                    if (need >= 3) { putchar(b3); }
                }
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
let n = readline(&line, cap);
print_line(&line, n);
