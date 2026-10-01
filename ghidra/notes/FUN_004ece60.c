
undefined4 __fastcall FUN_004ece60(uint *param_1)

{
  if (((*param_1 & 0xff000000) == 0) && ((*param_1 & 0xffffff) == 2)) {
    return 0;
  }
  return 1;
}

