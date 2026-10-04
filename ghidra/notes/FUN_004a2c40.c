
void __thiscall FUN_004a2c40(void *param_1,int param_2,LONG param_3,int param_4)

{
  uint uVar1;
  
  if (*(int *)(param_4 + 0x24) == 200) {
    uVar1 = *(uint *)((int)param_1 + 0x14c) | 1;
  }
  else {
    if (*(int *)(param_4 + 0x24) != 0xc9) goto LAB_004a2c6d;
    uVar1 = *(uint *)((int)param_1 + 0x14c) & 0xfffffffe;
  }
  *(uint *)((int)param_1 + 0x14c) = uVar1;
LAB_004a2c6d:
  FUN_004ac5c0(param_1,param_2,param_3,param_4);
  return;
}

