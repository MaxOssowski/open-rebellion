
undefined4 __cdecl FUN_00506ea0(int param_1)

{
  if (((0 < param_1) && (param_1 < 3)) && (DAT_006b2bb0 != 0)) {
    if (param_1 == 1) {
      return *(undefined4 *)(DAT_006b2bb0 + 0xc4);
    }
    return *(undefined4 *)(DAT_006b2bb0 + 200);
  }
  return 0;
}

