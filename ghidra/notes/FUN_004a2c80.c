
void __thiscall FUN_004a2c80(int param_1,int *param_2,int param_3)

{
  void *pvVar1;
  int iVar2;
  int *piVar3;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0063798b;
  pvStack_c = ExceptionList;
  piVar3 = (int *)0x0;
  ExceptionList = &pvStack_c;
  (**(code **)(*param_2 + 4))();
  if ((param_3 == *(int *)(param_1 + 0x160)) ||
     ((param_3 == 0 && ((*(byte *)(param_1 + 0x14c) & 1) == 0)))) {
    iVar2 = **(int **)(*(int *)(param_1 + 0x160) + 0xa0);
  }
  else {
    if ((param_3 != *(int *)(param_1 + 0x164)) &&
       ((param_3 != 0 || ((*(byte *)(param_1 + 0x14c) & 1) == 0)))) goto joined_r0x004a2cf8;
    iVar2 = **(int **)(*(int *)(param_1 + 0x164) + 0xa0);
  }
  piVar3 = (int *)(**(code **)(iVar2 + 8))();
joined_r0x004a2cf8:
  for (; piVar3 != (int *)0x0; piVar3 = (int *)(**(code **)(*piVar3 + 0xc))()) {
    if ((*(byte *)(piVar3 + 0xf) & 1) != 0) {
      pvVar1 = (void *)FUN_00618b70(0x20);
      uStack_4 = 0;
      if (pvVar1 == (void *)0x0) {
        pvVar1 = (void *)0x0;
      }
      else {
        pvVar1 = FUN_004f5b10(pvVar1,piVar3 + 0x1b,0);
      }
      uStack_4 = 0xffffffff;
      FUN_004f57b0(param_2,pvVar1);
    }
  }
  ExceptionList = pvStack_c;
  return;
}

