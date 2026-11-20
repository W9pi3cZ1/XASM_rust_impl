/* ==== _init @ 00101000 ==== */

int _init(EVP_PKEY_CTX *ctx)

{
  int iVar1;
  
  iVar1 = 0;
  if (PTR___gmon_start___00106fd0 != (undefined *)0x0) {
    iVar1 = (*(code *)PTR___gmon_start___00106fd0)();
  }
  return iVar1;
}


/* ==== FUN_00101020 @ 00101020 ==== */

void FUN_00101020(void)

{
  (*(code *)PTR_00106ff8)();
  return;
}


/* ==== putchar @ 00101030 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int putchar(int __c)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_putchar_00107000)();
  return iVar1;
}


/* ==== printf @ 00101040 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int printf(char *__format,...)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_printf_00107008)();
  return iVar1;
}


/* ==== fgetc @ 00101050 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int fgetc(FILE *__stream)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_fgetc_00107010)();
  return iVar1;
}


/* ==== getchar @ 00101060 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int getchar(void)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_getchar_00107018)();
  return iVar1;
}


/* ==== select @ 00101070 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int select(int __nfds,fd_set *__readfds,fd_set *__writefds,fd_set *__exceptfds,timeval *__timeout)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_select_00107020)();
  return iVar1;
}


/* ==== fseek @ 00101080 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int fseek(FILE *__stream,long __off,int __whence)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_fseek_00107028)();
  return iVar1;
}


/* ==== tcgetattr @ 00101090 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int tcgetattr(int __fd,termios *__termios_p)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_tcgetattr_00107030)();
  return iVar1;
}


/* ==== tcsetattr @ 001010a0 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int tcsetattr(int __fd,int __optional_actions,termios *__termios_p)

{
  int iVar1;
  
  iVar1 = (*(code *)PTR_tcsetattr_00107038)();
  return iVar1;
}


/* ==== fopen @ 001010b0 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

FILE * fopen(char *__filename,char *__modes)

{
  FILE *pFVar1;
  
  pFVar1 = (FILE *)(*(code *)PTR_fopen_00107040)();
  return pFVar1;
}


/* ==== __isoc99_scanf @ 001010c0 ==== */

void __isoc99_scanf(void)

{
  (*(code *)PTR___isoc99_scanf_00107048)();
  return;
}


/* ==== exit @ 001010d0 ==== */

/* WARNING: Unknown calling convention -- yet parameter storage is locked */

void exit(int __status)

{
  (*(code *)PTR_exit_00107050)();
  return;
}


/* ==== __cxa_finalize @ 001010e0 ==== */

void __cxa_finalize(void)

{
  (*(code *)PTR___cxa_finalize_00106fe0)();
  return;
}


/* ==== _start @ 001010f0 ==== */

void processEntry _start(undefined8 param_1,undefined8 param_2,undefined1 param_3)

{
  undefined1 auStack_8 [8];
  
  (*(code *)PTR___libc_start_main_00106fc0)(main,param_2,&param_3,0,0,param_1,auStack_8);
  do {
                    /* WARNING: Do nothing block with infinite loop */
  } while( true );
}


/* ==== deregister_tm_clones @ 00101120 ==== */

/* WARNING: Removing unreachable block (ram,0x00101133) */
/* WARNING: Removing unreachable block (ram,0x0010113f) */

void deregister_tm_clones(void)

{
  return;
}


/* ==== register_tm_clones @ 00101150 ==== */

/* WARNING: Removing unreachable block (ram,0x00101174) */
/* WARNING: Removing unreachable block (ram,0x00101180) */

void register_tm_clones(void)

{
  return;
}


/* ==== __do_global_dtors_aux @ 00101190 ==== */

void __do_global_dtors_aux(void)

{
  if (completed_0 != '\0') {
    return;
  }
  if (PTR___cxa_finalize_00106fe0 != (undefined *)0x0) {
    __cxa_finalize(__dso_handle);
  }
  deregister_tm_clones();
  completed_0 = 1;
  return;
}


/* ==== frame_dummy @ 001011d0 ==== */

void frame_dummy(void)

{
  register_tm_clones();
  return;
}


/* ==== terminal_lnbuf @ 001011d9 ==== */

void terminal_lnbuf(int param_1)

{
  termios local_88;
  termios local_48;
  
  tcgetattr(0,&local_48);
  local_88.c_iflag = local_48.c_iflag;
  local_88.c_oflag = local_48.c_oflag;
  local_88.c_line = local_48.c_line;
  local_88.c_cc[0] = local_48.c_cc[0];
  local_88.c_cc[1] = local_48.c_cc[1];
  local_88.c_cc[2] = local_48.c_cc[2];
  local_88.c_cc[3] = local_48.c_cc[3];
  local_88.c_cc[4] = local_48.c_cc[4];
  local_88.c_cc[5] = local_48.c_cc[5];
  local_88.c_cc[6] = local_48.c_cc[6];
  local_88.c_cc[7] = local_48.c_cc[7];
  local_88.c_cc[8] = local_48.c_cc[8];
  local_88.c_cc[9] = local_48.c_cc[9];
  local_88.c_cc[10] = local_48.c_cc[10];
  local_88.c_cc[0xb] = local_48.c_cc[0xb];
  local_88.c_cc[0xc] = local_48.c_cc[0xc];
  local_88.c_cc[0xd] = local_48.c_cc[0xd];
  local_88.c_cc[0xe] = local_48.c_cc[0xe];
  local_88.c_cc[0xf] = local_48.c_cc[0xf];
  local_88.c_cc[0x10] = local_48.c_cc[0x10];
  local_88.c_cc[0x11] = local_48.c_cc[0x11];
  local_88.c_cc[0x12] = local_48.c_cc[0x12];
  local_88.c_cc[0x13] = local_48.c_cc[0x13];
  local_88.c_cc[0x14] = local_48.c_cc[0x14];
  local_88.c_cc[0x15] = local_48.c_cc[0x15];
  local_88.c_cc[0x16] = local_48.c_cc[0x16];
  local_88.c_ispeed = local_48.c_ispeed;
  local_88.c_ospeed = local_48.c_ospeed;
  if (param_1 == 0) {
    local_88._8_8_ = local_48._8_8_ & 0xfffffffdffffffff;
  }
  else {
    local_88._8_8_ = local_48._8_8_ | 0x200000000;
  }
  tcsetattr(0,0,&local_88);
  return;
}


/* ==== terminal_echo @ 00101271 ==== */

void terminal_echo(int param_1)

{
  termios local_88;
  termios local_48;
  
  tcgetattr(0,&local_48);
  local_88.c_iflag = local_48.c_iflag;
  local_88.c_oflag = local_48.c_oflag;
  local_88.c_line = local_48.c_line;
  local_88.c_cc[0] = local_48.c_cc[0];
  local_88.c_cc[1] = local_48.c_cc[1];
  local_88.c_cc[2] = local_48.c_cc[2];
  local_88.c_cc[3] = local_48.c_cc[3];
  local_88.c_cc[4] = local_48.c_cc[4];
  local_88.c_cc[5] = local_48.c_cc[5];
  local_88.c_cc[6] = local_48.c_cc[6];
  local_88.c_cc[7] = local_48.c_cc[7];
  local_88.c_cc[8] = local_48.c_cc[8];
  local_88.c_cc[9] = local_48.c_cc[9];
  local_88.c_cc[10] = local_48.c_cc[10];
  local_88.c_cc[0xb] = local_48.c_cc[0xb];
  local_88.c_cc[0xc] = local_48.c_cc[0xc];
  local_88.c_cc[0xd] = local_48.c_cc[0xd];
  local_88.c_cc[0xe] = local_48.c_cc[0xe];
  local_88.c_cc[0xf] = local_48.c_cc[0xf];
  local_88.c_cc[0x10] = local_48.c_cc[0x10];
  local_88.c_cc[0x11] = local_48.c_cc[0x11];
  local_88.c_cc[0x12] = local_48.c_cc[0x12];
  local_88.c_cc[0x13] = local_48.c_cc[0x13];
  local_88.c_cc[0x14] = local_48.c_cc[0x14];
  local_88.c_cc[0x15] = local_48.c_cc[0x15];
  local_88.c_cc[0x16] = local_48.c_cc[0x16];
  local_88.c_ispeed = local_48.c_ispeed;
  local_88.c_ospeed = local_48.c_ospeed;
  if (param_1 == 0) {
    local_88._8_8_ = local_48._8_8_ & 0xfffffff7ffffffff;
  }
  else {
    local_88._8_8_ = local_48._8_8_ | 0x800000000;
  }
  tcsetattr(0,0,&local_88);
  return;
}


/* ==== gotoxy @ 00101309 ==== */

void gotoxy(uint param_1,uint param_2)

{
  printf("\x1b[%d;%df",(ulong)param_2,(ulong)param_1);
  return;
}


/* ==== _gotoxy @ 00101336 ==== */

void _gotoxy(undefined4 param_1,undefined4 param_2)

{
  gotoxy(param_1,param_2);
  return;
}


/* ==== clrscr @ 00101356 ==== */

void clrscr(void)

{
  printf("\x1b[2J\x1b[0;0f");
  return;
}


/* ==== _clrscr @ 00101371 ==== */

void _clrscr(void)

{
  clrscr();
  return;
}


/* ==== getch @ 00101382 ==== */

int getch(void)

{
  int iVar1;
  
  terminal_lnbuf(0);
  terminal_echo(0);
  iVar1 = getchar();
  terminal_lnbuf(1);
  terminal_echo(1);
  return iVar1;
}


/* ==== _getch @ 001013c2 ==== */

void _getch(void)

{
  getch();
  return;
}


/* ==== getche @ 001013d2 ==== */

int getche(void)

{
  int iVar1;
  
  terminal_lnbuf(0);
  iVar1 = getchar();
  terminal_lnbuf(1);
  return iVar1;
}


/* ==== _getche @ 001013fe ==== */

void _getche(void)

{
  getche();
  return;
}


/* ==== kbhit @ 0010140e ==== */

bool kbhit(void)

{
  ulong uVar1;
  timeval local_b8;
  fd_set local_a8;
  fd_set *local_28;
  uint local_1c;
  
  terminal_lnbuf(0);
  terminal_echo(0);
  local_b8.tv_sec = 0;
  local_b8.tv_usec = 0;
  local_28 = &local_a8;
  for (local_1c = 0; local_1c < 0x10; local_1c = local_1c + 1) {
    local_28->fds_bits[local_1c] = 0;
  }
  local_a8.fds_bits[0] = local_a8.fds_bits[0] | 1;
  select(1,&local_a8,(fd_set *)0x0,(fd_set *)0x0,&local_b8);
  uVar1 = local_a8.fds_bits[0] & 1;
  terminal_lnbuf(1);
  terminal_echo(1);
  return uVar1 != 0;
}


/* ==== _kbhit @ 001014da ==== */

void _kbhit(void)

{
  kbhit();
  return;
}


/* ==== main @ 001014ea ==== */

undefined8 main(void)

{
  undefined4 uVar1;
  undefined4 uVar2;
  undefined4 uVar3;
  int iVar4;
  int iVar5;
  undefined4 uVar6;
  
  fp = fopen("./disk.bin","r");
  for (local = 0; local < 0xfffff; local = local + 1) {
    fseek(fp,(long)local,0);
    iVar4 = local;
    iVar5 = fgetc(fp);
    (&memory)[iVar4] = iVar5;
  }
  local = 0;
  do {
    if (0xffffe < local) {
      return 0;
    }
    if ((&memory)[local] == 0xff) {
      if (DAT_001070c4 == 0) {
        DAT_001070c4 = 1;
      }
      else {
        DAT_001070c4 = 0;
      }
    }
    if (DAT_001070c4 != 1) {
      if ((&memory)[local] == 1) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        DAT_00107110 = (&memory)[local + 5];
        DAT_00107114 = (&memory)[local + 6];
        DAT_00107118 = (&memory)[local + 7];
        local = local + 8;
        DAT_0010711c = (&memory)[local];
        iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
        iVar4 = (&memory)[iVar4];
        iVar5 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        (&memory)[iVar5] = iVar4 + (&memory)[iVar5];
      }
      else if ((&memory)[local] == 2) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        DAT_00107110 = (&memory)[local + 5];
        DAT_00107114 = (&memory)[local + 6];
        DAT_00107118 = (&memory)[local + 7];
        local = local + 8;
        DAT_0010711c = (&memory)[local];
        iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
        iVar4 = (&memory)[iVar4];
        iVar5 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        (&memory)[iVar5] = (&memory)[iVar5] - iVar4;
      }
      else if ((&memory)[local] == 3) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        DAT_00107110 = (&memory)[local + 5];
        DAT_00107114 = (&memory)[local + 6];
        DAT_00107118 = (&memory)[local + 7];
        local = local + 8;
        DAT_0010711c = (&memory)[local];
        iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
        iVar4 = (&memory)[iVar4];
        iVar5 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        (&memory)[iVar5] = (&memory)[iVar5] * iVar4;
      }
      else if ((&memory)[local] == 4) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        DAT_00107110 = (&memory)[local + 5];
        DAT_00107114 = (&memory)[local + 6];
        DAT_00107118 = (&memory)[local + 7];
        local = local + 8;
        DAT_0010711c = (&memory)[local];
        iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
        iVar4 = (&memory)[iVar4];
        iVar5 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        (&memory)[iVar5] = (int)(&memory)[iVar5] / iVar4;
      }
      else if ((&memory)[local] == 0x11) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        DAT_00107110 = (&memory)[local + 5];
        DAT_00107114 = (&memory)[local + 6];
        DAT_00107118 = (&memory)[local + 7];
        DAT_0010711c = (&memory)[local + 8];
        DAT_00107120 = (&memory)[local + 9];
        DAT_00107124 = (&memory)[local + 10];
        DAT_00107128 = (&memory)[local + 0xb];
        local = local + 0xc;
        DAT_0010712c = (&memory)[local];
        iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        if (((&memory)[iVar4] == 0) ||
           (iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c),
           (&memory)[iVar4] == 0)) {
          iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
          (&memory)[iVar4] = 0;
        }
        else {
          iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
          (&memory)[iVar4] = 1;
        }
      }
      else if ((&memory)[local] == 0x12) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        DAT_00107110 = (&memory)[local + 5];
        DAT_00107114 = (&memory)[local + 6];
        DAT_00107118 = (&memory)[local + 7];
        DAT_0010711c = (&memory)[local + 8];
        DAT_00107120 = (&memory)[local + 9];
        DAT_00107124 = (&memory)[local + 10];
        DAT_00107128 = (&memory)[local + 0xb];
        local = local + 0xc;
        DAT_0010712c = (&memory)[local];
        iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        if (((&memory)[iVar4] == 0) &&
           (iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c),
           (&memory)[iVar4] == 0)) {
          iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
          (&memory)[iVar4] = 0;
        }
        else {
          iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
          (&memory)[iVar4] = 1;
        }
      }
      else if ((&memory)[local] == 0x13) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        DAT_00107110 = (&memory)[local + 5];
        DAT_00107114 = (&memory)[local + 6];
        DAT_00107118 = (&memory)[local + 7];
        DAT_0010711c = (&memory)[local + 8];
        DAT_00107120 = (&memory)[local + 9];
        DAT_00107124 = (&memory)[local + 10];
        DAT_00107128 = (&memory)[local + 0xb];
        local = local + 0xc;
        DAT_0010712c = (&memory)[local];
        iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        iVar4 = (&memory)[iVar4];
        iVar5 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
        if (iVar4 == (&memory)[iVar5]) {
          iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
          (&memory)[iVar4] = 0;
        }
        else {
          iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
          (&memory)[iVar4] = 1;
        }
      }
      else if ((&memory)[local] == 0x21) {
        cache = (&memory)[local + 1];
        DAT_00107104 = (&memory)[local + 2];
        DAT_00107108 = (&memory)[local + 3];
        DAT_0010710c = (&memory)[local + 4];
        uVar6 = (&memory)[local + 5];
        uVar1 = (&memory)[local + 6];
        uVar2 = (&memory)[local + 7];
        local = local + 8;
        uVar3 = (&memory)[local];
        DAT_00107110 = uVar6;
        DAT_00107114 = uVar1;
        DAT_00107118 = uVar2;
        DAT_0010711c = uVar3;
        iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
        uVar6 = hexcat(uVar6,uVar1,uVar2,uVar3);
        (&memory)[iVar4] = uVar6;
      }
      else {
        if ((&memory)[local] == 0x22) {
          cache = (&memory)[local + 1];
          DAT_00107104 = (&memory)[local + 2];
          DAT_00107108 = (&memory)[local + 3];
          DAT_0010710c = (&memory)[local + 4];
          DAT_00107110 = (&memory)[local + 5];
          DAT_00107114 = (&memory)[local + 6];
          DAT_00107118 = (&memory)[local + 7];
          local = local + 8;
          DAT_0010711c = (&memory)[local];
          iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
          iVar5 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
          (&memory)[iVar5] = (&memory)[iVar4];
        }
        iVar4 = local;
        if ((&memory)[local] == 0x23) {
          cache = (&memory)[local + 1];
          DAT_00107104 = (&memory)[local + 2];
          DAT_00107108 = (&memory)[local + 3];
          local = local + 4;
          DAT_0010710c = (&memory)[local];
          iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
          local = (&memory)[iVar4] + -1;
        }
        else if ((&memory)[local] == 0x24) {
          cache = (&memory)[local + 1];
          DAT_00107104 = (&memory)[local + 2];
          DAT_00107108 = (&memory)[local + 3];
          local = local + 4;
          DAT_0010710c = (&memory)[local];
          iVar5 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
          (&memory)[iVar5] = iVar4 + 5;
        }
        else {
          if ((&memory)[local] == 0x25) {
            cache = (&memory)[local + 1];
            DAT_00107104 = (&memory)[local + 2];
            DAT_00107108 = (&memory)[local + 3];
            DAT_0010710c = (&memory)[local + 4];
            DAT_00107110 = (&memory)[local + 5];
            DAT_00107114 = (&memory)[local + 6];
            DAT_00107118 = (&memory)[local + 7];
            DAT_0010711c = (&memory)[local + 8];
            DAT_00107120 = (&memory)[local + 9];
            DAT_00107124 = (&memory)[local + 10];
            DAT_00107128 = (&memory)[local + 0xb];
            local = local + 0xc;
            DAT_0010712c = (&memory)[local];
            iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
            iVar4 = (&memory)[iVar4];
            iVar5 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
            if ((int)(&memory)[iVar5] < iVar4) {
              iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
              local = (&memory)[iVar4] + -1;
            }
          }
          if ((&memory)[local] == 0x26) {
            cache = (&memory)[local + 1];
            DAT_00107104 = (&memory)[local + 2];
            DAT_00107108 = (&memory)[local + 3];
            DAT_0010710c = (&memory)[local + 4];
            DAT_00107110 = (&memory)[local + 5];
            DAT_00107114 = (&memory)[local + 6];
            DAT_00107118 = (&memory)[local + 7];
            DAT_0010711c = (&memory)[local + 8];
            DAT_00107120 = (&memory)[local + 9];
            DAT_00107124 = (&memory)[local + 10];
            DAT_00107128 = (&memory)[local + 0xb];
            local = local + 0xc;
            DAT_0010712c = (&memory)[local];
            iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
            iVar4 = (&memory)[iVar4];
            iVar5 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
            if (iVar4 < (int)(&memory)[iVar5]) {
              iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
              local = (&memory)[iVar4] + -1;
            }
          }
          if ((&memory)[local] == 0x27) {
            cache = (&memory)[local + 1];
            DAT_00107104 = (&memory)[local + 2];
            DAT_00107108 = (&memory)[local + 3];
            DAT_0010710c = (&memory)[local + 4];
            DAT_00107110 = (&memory)[local + 5];
            DAT_00107114 = (&memory)[local + 6];
            DAT_00107118 = (&memory)[local + 7];
            DAT_0010711c = (&memory)[local + 8];
            DAT_00107120 = (&memory)[local + 9];
            DAT_00107124 = (&memory)[local + 10];
            DAT_00107128 = (&memory)[local + 0xb];
            local = local + 0xc;
            DAT_0010712c = (&memory)[local];
            iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
            iVar4 = (&memory)[iVar4];
            iVar5 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
            if (iVar4 == (&memory)[iVar5]) {
              iVar4 = hexcat(DAT_00107120,DAT_00107124,DAT_00107128,DAT_0010712c);
              local = (&memory)[iVar4] + -1;
            }
          }
          else if ((&memory)[local] == 0x28) {
            cache = (&memory)[local + 1];
            DAT_00107104 = (&memory)[local + 2];
            DAT_00107108 = (&memory)[local + 3];
            DAT_0010710c = (&memory)[local + 4];
            DAT_00107110 = (&memory)[local + 5];
            DAT_00107114 = (&memory)[local + 6];
            DAT_00107118 = (&memory)[local + 7];
            local = local + 8;
            DAT_0010711c = (&memory)[local];
            DAT_00107120 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
            DAT_00107124 = (&memory)[DAT_00107120];
            (&memory)[DAT_00107120] = 0;
            iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
            DAT_00107120 = DAT_00107120 - (&memory)[iVar4];
            (&memory)[DAT_00107120] = DAT_00107124;
          }
          else if ((&memory)[local] == 0x29) {
            cache = (&memory)[local + 1];
            DAT_00107104 = (&memory)[local + 2];
            DAT_00107108 = (&memory)[local + 3];
            DAT_0010710c = (&memory)[local + 4];
            DAT_00107110 = (&memory)[local + 5];
            DAT_00107114 = (&memory)[local + 6];
            DAT_00107118 = (&memory)[local + 7];
            local = local + 8;
            DAT_0010711c = (&memory)[local];
            DAT_00107120 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
            DAT_00107124 = (&memory)[DAT_00107120];
            (&memory)[DAT_00107120] = 0;
            iVar4 = hexcat(DAT_00107110,DAT_00107114,DAT_00107118,DAT_0010711c);
            DAT_00107120 = DAT_00107120 + (&memory)[iVar4];
            (&memory)[DAT_00107120] = DAT_00107124;
          }
          else {
            if ((&memory)[local] == 0x2a) {
                    /* WARNING: Subroutine does not return */
              exit(0);
            }
            if ((&memory)[local] == 0x31) {
              cache = (&memory)[local + 1];
              DAT_00107104 = (&memory)[local + 2];
              DAT_00107108 = (&memory)[local + 3];
              local = local + 4;
              DAT_0010710c = (&memory)[local];
              iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
              putchar((&memory)[(int)(&memory)[iVar4]]);
            }
            else if ((&memory)[local] == 0x32) {
              cache = (&memory)[local + 1];
              DAT_00107104 = (&memory)[local + 2];
              DAT_00107108 = (&memory)[local + 3];
              local = local + 4;
              DAT_0010710c = (&memory)[local];
              iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
              printf("%d",(ulong)(uint)(&memory)[(int)(&memory)[iVar4]]);
            }
            else if ((&memory)[local] == 0x33) {
              cache = (&memory)[local + 1];
              DAT_00107104 = (&memory)[local + 2];
              DAT_00107108 = (&memory)[local + 3];
              local = local + 4;
              DAT_0010710c = (&memory)[local];
              iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
              printf("%x",(ulong)(uint)(&memory)[(int)(&memory)[iVar4]]);
            }
            else if ((&memory)[local] == 0x34) {
              cache = (&memory)[local + 1];
              DAT_00107104 = (&memory)[local + 2];
              DAT_00107108 = (&memory)[local + 3];
              local = local + 4;
              DAT_0010710c = (&memory)[local];
              iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
              uVar6 = getch();
              (&memory)[iVar4] = uVar6;
            }
            else if ((&memory)[local] == 0x35) {
              cache = (&memory)[local + 1];
              DAT_00107104 = (&memory)[local + 2];
              DAT_00107108 = (&memory)[local + 3];
              local = local + 4;
              DAT_0010710c = (&memory)[local];
              iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
              __isoc99_scanf(&DAT_00105025,&memory + iVar4);
            }
            else if ((&memory)[local] == 0x36) {
              cache = (&memory)[local + 1];
              DAT_00107104 = (&memory)[local + 2];
              DAT_00107108 = (&memory)[local + 3];
              local = local + 4;
              DAT_0010710c = (&memory)[local];
              iVar4 = hexcat(cache,DAT_00107104,DAT_00107108,DAT_0010710c);
              __isoc99_scanf(&DAT_00105028,&memory + iVar4);
            }
          }
        }
      }
    }
    local = local + 1;
  } while( true );
}


/* ==== hexcat @ 001044d3 ==== */

int hexcat(int param_1,int param_2,int param_3,int param_4)

{
  return param_4 + (param_3 + (param_2 + param_1 * 0x100) * 0x100) * 0x100;
}


/* ==== _fini @ 00104508 ==== */

void _fini(void)

{
  return;
}


/* ==== putchar @ 00508000 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int putchar(int __c)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* putchar@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== __libc_start_main @ 00508008 ==== */

/* WARNING: Control flow encountered bad instruction data */

void __libc_start_main(void)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* __libc_start_main@GLIBC_2.34 */
  halt_baddata();
}


/* ==== _ITM_deregisterTMCloneTable @ 00508010 ==== */

/* WARNING: Control flow encountered bad instruction data */

void _ITM_deregisterTMCloneTable(void)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}


/* ==== printf @ 00508018 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int printf(char *__format,...)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* printf@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== fgetc @ 00508020 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int fgetc(FILE *__stream)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* fgetc@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== getchar @ 00508028 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int getchar(void)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* getchar@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== __gmon_start__ @ 00508030 ==== */

/* WARNING: Control flow encountered bad instruction data */

void __gmon_start__(void)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}


/* ==== select @ 00508038 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int select(int __nfds,fd_set *__readfds,fd_set *__writefds,fd_set *__exceptfds,timeval *__timeout)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* select@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== fseek @ 00508040 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int fseek(FILE *__stream,long __off,int __whence)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* fseek@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== tcgetattr @ 00508048 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int tcgetattr(int __fd,termios *__termios_p)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* tcgetattr@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== tcsetattr @ 00508050 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

int tcsetattr(int __fd,int __optional_actions,termios *__termios_p)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* tcsetattr@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== fopen @ 00508058 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

FILE * fopen(char *__filename,char *__modes)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* fopen@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== __isoc99_scanf @ 00508060 ==== */

/* WARNING: Control flow encountered bad instruction data */

void __isoc99_scanf(void)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* __isoc99_scanf@GLIBC_2.7 */
  halt_baddata();
}


/* ==== exit @ 00508068 ==== */

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Unknown calling convention -- yet parameter storage is locked */

void exit(int __status)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* exit@GLIBC_2.2.5 */
  halt_baddata();
}


/* ==== _ITM_registerTMCloneTable @ 00508070 ==== */

/* WARNING: Control flow encountered bad instruction data */

void _ITM_registerTMCloneTable(void)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}


/* ==== __cxa_finalize @ 00508078 ==== */

/* WARNING: Control flow encountered bad instruction data */

void __cxa_finalize(void)

{
                    /* WARNING: Bad instruction - Truncating control flow here */
                    /* __cxa_finalize@GLIBC_2.2.5 */
  halt_baddata();
}


