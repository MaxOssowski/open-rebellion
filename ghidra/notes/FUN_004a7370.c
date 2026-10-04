
void __fastcall FUN_004a7370(int *param_1)

{
  if ((param_1[0x53] & 0x10000000U) != 0) {
    param_1[0x53] = param_1[0x53] & 0xefffffff;
    FUN_004a3340(param_1);
    return;
  }
  return;
}

