
undefined4 __thiscall FUN_0052e530(void *this,int param_1,undefined4 *param_2)

{
  int iVar1;
  
  if ((((*(uint *)(param_1 + 0x50) & 0x40) != 0) &&
      (((*(uint *)(param_1 + 0x24) ^ *(uint *)((int)this + 0x24)) & 0xc0) == 0)) &&
     ((*(uint *)(param_1 + 0x50) & 4) != 0)) {
    iVar1 = FUN_004f2990(param_1);
    if (iVar1 != 0) {
      *param_2 = 1;
      return 1;
    }
  }
  *param_2 = 0;
  return 1;
}

