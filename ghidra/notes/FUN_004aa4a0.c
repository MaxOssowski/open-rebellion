
undefined4 __fastcall FUN_004aa4a0(int *param_1)

{
  int *piVar1;
  uint uVar2;
  
  piVar1 = FUN_004a25c0(param_1);
  if (piVar1 == (int *)0x0) {
    return 0;
  }
  uVar2 = (uint)piVar1[9] >> 6 & 3;
  if (uVar2 != 1) {
    if (uVar2 != 2) {
      return 0x2d0f;
    }
    return 0x2d0e;
  }
  return 0x2d0d;
}

