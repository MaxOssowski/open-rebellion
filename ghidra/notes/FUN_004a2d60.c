
void __thiscall FUN_004a2d60(int param_1,uint *param_2)

{
  bool bVar1;
  
  if ((*param_2 >> 0x18 < 0x14) || (0x1f < *param_2 >> 0x18)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  FUN_00619730();
  if (bVar1) {
    *(uint *)(param_1 + 0x14c) = *(uint *)(param_1 + 0x14c) | 0x10000000;
    return;
  }
  if ((*param_2 >> 0x18 < 8) || (0xf < *param_2 >> 0x18)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  FUN_00619730();
  if (!bVar1) {
    if ((*param_2 >> 0x18 < 0x10) || (0x13 < *param_2 >> 0x18)) {
      bVar1 = false;
    }
    else {
      bVar1 = true;
    }
    FUN_00619730();
    if (!bVar1) {
      if ((*param_2 >> 0x18 < 0x30) || (0x3f < *param_2 >> 0x18)) {
        bVar1 = false;
      }
      else {
        bVar1 = true;
      }
      FUN_00619730();
      if (!bVar1) {
        return;
      }
    }
  }
  *(uint *)(param_1 + 0x14c) = *(uint *)(param_1 + 0x14c) | 0x10000000;
  return;
}

