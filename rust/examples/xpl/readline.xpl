// Minimal raw-terminal readline.
//
// XPL has no string values.  The line is stored as bytes in a caller-provided
// array; &line is an i32 address passed by value, and writes through poke()
// make it behave like a reference to the shared buffer.

let line[128];
let cap = 128;

fn readline(addr, max) {
    let n = 0;
    let done = 0;

    while (done == 0) {
        let c = read_char();

        if (c == -1) {
            // EOF
            done = 1;
        } else if (c == 10 || c == 13) {
            // LF or CR (raw TTY usually sends LF after ICRNL)
            done = 1;
            println();
        } else if (c == 8 || c == 127) {
            // Backspace / DEL: erase the last byte on screen.
            if (n > 0) {
                n = n - 1;
                putchar(8);
                putchar(32);
                putchar(8);
            }
        } else if (c >= 32 && c < 127) {
            // Printable ASCII; UTF-8 and escape sequences need more work.
            if (n < max) {
                poke(addr + n, c);
                n = n + 1;
                putchar(c);
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
