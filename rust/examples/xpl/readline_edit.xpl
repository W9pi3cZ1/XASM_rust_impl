// A small UTF-8 line editor for a raw TTY.
//
// Supports: printable text, UTF-8 codepoints, Left/Right, Home/End,
// Backspace, Delete, Ctrl-A/E/U/K, and insert-at-cursor.
// Up/Down history, selection, and wrapping are not implemented.
//
// The buffer stores UTF-8 bytes; n is a byte count.  Cursor positions are
// byte offsets, always on codepoint boundaries.

let line[256];
let cap = 256;

fn is_cont(b) {
    return b >= 0x80 && b < 0xc0;
}

fn seq_len(b0) {
    if (b0 < 0x80) { return 1; }
    if (b0 >= 0xc2 && b0 < 0xe0) { return 2; }
    if (b0 >= 0xe0 && b0 < 0xf0) { return 3; }
    if (b0 >= 0xf0 && b0 <= 0xf4) { return 4; }
    return 0;
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
    let n = seq_len(b0);
    if (n == 2) {
        let b1 = peek(addr + start + 1);
        return utf8_width((b0 - 0xc0) * 64 + (b1 - 0x80));
    }
    if (n == 3) {
        let b1 = peek(addr + start + 1);
        let b2 = peek(addr + start + 2);
        return utf8_width((b0 - 0xe0) * 4096 + (b1 - 0x80) * 64 + (b2 - 0x80));
    }
    if (n == 4) {
        let b1 = peek(addr + start + 1);
        let b2 = peek(addr + start + 2);
        let b3 = peek(addr + start + 3);
        return utf8_width(
            (b0 - 0xf0) * 262144 + (b1 - 0x80) * 4096 + (b2 - 0x80) * 64 + (b3 - 0x80)
        );
    }
    return 1;
}

fn prev_start(addr, pos) {
    let p = pos - 1;
    while (p > 0 && is_cont(peek(addr + p))) {
        p = p - 1;
    }
    return p;
}

fn next_end(addr, pos, len) {
    let n = seq_len(peek(addr + pos));
    if (n <= 0) {
        return pos + 1;
    }
    if (pos + n > len) {
        return len;
    }
    return pos + n;
}

fn line_width(addr, start, len) {
    let i = start;
    let w = 0;
    while (i < len) {
        let n = seq_len(peek(addr + i));
        if (n <= 0) {
            i = i + 1;
            w = w + 1;
        } else {
            w = w + width_at(addr, i);
            i = i + n;
        }
    }
    return w;
}

fn redraw(addr, len, cur) {
    putchar(13);
    print("> ");
    let i = 0;
    while (i < len) {
        putchar(peek(addr + i));
        i = i + 1;
    }
    print("\x1b[K");

    let w = line_width(addr, cur, len);
    if (w > 0) {
        print("\x1b[");
        print(w);
        putchar(68); // 'D'
    }
}

fn delete_at(addr, len, start, count) {
    let i = start;
    while (i + count < len) {
        poke(addr + i, peek(addr + i + count));
        i = i + 1;
    }
    return len - count;
}

fn readline(addr, max) {
    let len = 0;
    let cur = 0;
    let done = 0;

    while (done == 0) {
        let b0 = read_char();

        if (b0 == -1) {
            done = 1;
        } else if (b0 == 10 || b0 == 13) {
            done = 1;
            println();
        } else if (b0 == 8 || b0 == 127) {
            if (cur > 0) {
                let start = prev_start(addr, cur);
                len = delete_at(addr, len, start, cur - start);
                cur = start;
                redraw(addr, len, cur);
            }
        } else if (b0 == 1) { // Ctrl-A
            cur = 0;
            redraw(addr, len, cur);
        } else if (b0 == 5) { // Ctrl-E
            cur = len;
            redraw(addr, len, cur);
        } else if (b0 == 11) { // Ctrl-K
            if (cur < len) {
                len = cur;
                redraw(addr, len, cur);
            }
        } else if (b0 == 21) { // Ctrl-U
            len = 0;
            cur = 0;
            redraw(addr, len, cur);
        } else if (b0 == 27) { // ESC [ ... / ESC O ...
            let b1 = read_char();
            if (b1 == -1) {
                done = 1;
            } else if (b1 == 91 || b1 == 79) {
                let b2 = read_char();
                if (b2 == 68) { // Left
                    if (cur > 0) {
                        cur = prev_start(addr, cur);
                        redraw(addr, len, cur);
                    }
                } else if (b2 == 67) { // Right
                    if (cur < len) {
                        cur = next_end(addr, cur, len);
                        redraw(addr, len, cur);
                    }
                } else if (b2 == 72) { // Home
                    cur = 0;
                    redraw(addr, len, cur);
                } else if (b2 == 70) { // End
                    cur = len;
                    redraw(addr, len, cur);
                } else if (b2 == 51) { // Delete: ESC [ 3 ~
                    let b3 = read_char();
                    if (b3 == 126 && cur < len) {
                        let end = next_end(addr, cur, len);
                        len = delete_at(addr, len, cur, end - cur);
                        redraw(addr, len, cur);
                    }
                } else if (b2 == 49 || b2 == 55) { // Home: ESC [ 1 ~ / 7 ~
                    let b3 = read_char();
                    if (b3 == 126) {
                        cur = 0;
                        redraw(addr, len, cur);
                    }
                } else if (b2 == 52 || b2 == 56) { // End: ESC [ 4 ~ / 8 ~
                    let b3 = read_char();
                    if (b3 == 126) {
                        cur = len;
                        redraw(addr, len, cur);
                    }
                } else {
                    // Consume an unknown CSI sequence up to its final byte.
                    while (b2 != -1 && b2 != 126 && (b2 < 64 || b2 > 126)) {
                        b2 = read_char();
                    }
                }
            }
        } else if (b0 >= 32 && b0 < 127) {
            if (len + 1 <= max) {
                let i = len;
                while (i > cur) {
                    poke(addr + i, peek(addr + i - 1));
                    i = i - 1;
                }
                poke(addr + cur, b0);
                len = len + 1;
                cur = cur + 1;
                redraw(addr, len, cur);
            }
        } else if (b0 >= 0xc2 && b0 <= 0xf4) {
            let n = seq_len(b0);
            let b1 = 0;
            let b2 = 0;
            let b3 = 0;
            let valid = 1;

            if (n >= 2) {
                b1 = read_char();
                if (!is_cont(b1)) { valid = 0; }
            }
            if (n >= 3) {
                b2 = read_char();
                if (!is_cont(b2)) { valid = 0; }
            }
            if (n >= 4) {
                b3 = read_char();
                if (!is_cont(b3)) { valid = 0; }
            }

            if (valid == 1) {
                let cp = 0;
                if (n == 2) {
                    cp = (b0 - 0xc0) * 64 + (b1 - 0x80);
                } else if (n == 3) {
                    cp = (b0 - 0xe0) * 4096 + (b1 - 0x80) * 64 + (b2 - 0x80);
                } else {
                    cp = (b0 - 0xf0) * 262144 + (b1 - 0x80) * 4096
                        + (b2 - 0x80) * 64 + (b3 - 0x80);
                }

                if (n == 3 && cp < 0x800) { valid = 0; }
                if (n == 4 && cp < 0x10000) { valid = 0; }
                if (cp >= 0xd800 && cp <= 0xdfff) { valid = 0; }
                if (cp > 0x10ffff) { valid = 0; }
            }

            if (valid == 1 && len + n <= max) {
                let i = len;
                while (i > cur) {
                    poke(addr + i + n - 1, peek(addr + i - 1));
                    i = i - 1;
                }
                poke(addr + cur, b0);
                if (n >= 2) { poke(addr + cur + 1, b1); }
                if (n >= 3) { poke(addr + cur + 2, b2); }
                if (n >= 4) { poke(addr + cur + 3, b3); }
                len = len + n;
                cur = cur + n;
                redraw(addr, len, cur);
            }
        }
    }

    return len;
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
