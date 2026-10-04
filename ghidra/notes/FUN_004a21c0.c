
undefined4 __fastcall FUN_004a21c0(int *param_1)

{
  int *piVar1;
  uint uVar2;
  
  piVar1 = FUN_004a25c0(param_1);
  if (piVar1 == (int *)0x0) {
    return 0;
  }
  uVar2 = FUN_004a1f60((int)piVar1,*(int *)(param_1[0x53] + 0x9c));
  if (uVar2 != 1) {
    if (uVar2 != 2) {
      return 0x2d15;
    }
    return 0x2d14;
  }
  return 0x2d13;
}

