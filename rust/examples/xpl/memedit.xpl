// Minimal interactive XASM memory editor.
// h/l: move cursor, j/k: move one row, 0-9a-f: edit byte, q: quit.
// Run with xasm-run on a TTY to use raw single-key input.

// Alternate screen: do not scribble over the shell prompt.
print("\x1b[?1049h");
print("\x1b[?25l");
print("\x1b[2J\x1b[H");

let addr = 0;
let cursor = 0;
let running = true;

while (running) {
    print("\x1b[H\x1b[2J");
    print("XASM memory editor    addr=");
    puthex(addr);
    print("  h/l move  j/k row  0-9a-f edit  q quit\n");

    let i = 0;
    while (i < 16) {
        if (i == cursor) {
            print("\x1b[7m");
        }
        let b = peek(addr + i) % 256;
        if (b < 0) {
            b = b + 256;
        }
        puthex(b / 16);
        puthex(b % 16);
        if (i == cursor) {
            print("\x1b[0m");
        }
        print(" ");
        i = i + 1;
    }
    println();

    print("value=");
    puthex(peek(addr + cursor));
    print("  key: ");

    let key = read_char();

    if (key == 113) {
        running = false;
    } else if (key == 104) {
        if (cursor > 0) {
            cursor = cursor - 1;
        }
    } else if (key == 108) {
        if (cursor < 15) {
            cursor = cursor + 1;
        }
    } else if (key == 106) {
        addr = addr + 16;
        if (addr > 0xfffee) {
            addr = 0xfffee;
        }
    } else if (key == 107) {
        if (addr >= 16) {
            addr = addr - 16;
        }
    }

    let nib = -1;
    if (key >= 48 && key <= 57) {
        nib = key - 48;
    } else if (key >= 97 && key <= 102) {
        nib = key - 87;
    } else if (key >= 65 && key <= 70) {
        nib = key - 55;
    }
    if (nib >= 0) {
        let key2 = read_char();
        let nib2 = -1;
        if (key2 >= 48 && key2 <= 57) {
            nib2 = key2 - 48;
        } else if (key2 >= 97 && key2 <= 102) {
            nib2 = key2 - 87;
        } else if (key2 >= 65 && key2 <= 70) {
            nib2 = key2 - 55;
        }
        if (nib2 >= 0) {
            poke(addr + cursor, nib * 16 + nib2);
            if (cursor < 15) {
                cursor = cursor + 1;
            }
        }
    }
}

print("\x1b[?25h");
print("\x1b[?1049l");
