// Compute pi with the classic integer spigot algorithm.
// It only uses i32 cells: no floating point and no big integers.
// Every pass prints pi to a larger number of digits, then starts over.

let digits[200];

fn print4(n) {
    print(n / 1000);
    print((n / 100) % 10);
    print((n / 10) % 10);
    print(n % 10);
}

fn print3(n) {
    print(n / 100);
    print((n / 10) % 10);
    print(n % 10);
}

fn spigot(cells) {
    let i = 0;
    while (i < cells) {
        digits[i] = 2000;
        i = i + 1;
    }

    let c = cells;
    let e = 0;
    let f = 10000;
    let first = 1;

    while (c > 0) {
        let d = 0;
        let g = c * 2;
        let b = c;

        while (b > 0) {
            d = d + digits[b - 1] * f;
            g = g - 1;
            digits[b - 1] = d % g;
            d = d / g;
            g = g - 1;
            b = b - 1;
            if (b > 0) {
                d = d * b;
            }
        }

        c = c - 14;
        let out = e + d / f;

        if (first == 1) {
            // The first group is 3141; "3." was already printed.
            print3(out % 1000);
            first = 0;
        } else {
            print4(out);
        }
        e = d % f;
    }
}

let precision = 120;
while (true) {
    print("pi = 3.");
    spigot(precision);
    println();
    precision = precision + 20;
    if (precision > 160) {
        precision = 120;
    }
}
