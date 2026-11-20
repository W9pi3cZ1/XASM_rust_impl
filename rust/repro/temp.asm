// Minimal reproducer for the original XASM1 dispatch fallthrough bug.
// `copy` reads 8 operand bytes, so its last operand byte is the low byte of
// the source address (0x23 here).  The VM then falls through to the
// `if (memory[local] == 0x23)` goto check and executes a bogus goto.
copy 0x00 0x00 0x01 0x00 0x00 0x00 0x00 0x23
exit
