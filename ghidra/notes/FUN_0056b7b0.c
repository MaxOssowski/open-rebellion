
bool __thiscall FUN_0056b7b0(void *param_1,int *param_2,undefined4 *param_3)

{
  int iVar1;
  uint uVar2;
  int iVar3;
  bool bVar4;
  void *pvStack_4;
  
  pvStack_4 = param_1;
  iVar1 = FUN_00586720(param_1,(int *)&pvStack_4);
  bVar4 = iVar1 != 0;
  if (pvStack_4 != (void *)0x0) {
    uVar2 = *(uint *)((int)param_1 + 0x24) >> 6 & 3;
    if (uVar2 == 3) {
      iVar1 = 3;
    }
    else {
      iVar1 = 2 - (uint)(uVar2 != 1);
    }
    iVar1 = FUN_00507270(pvStack_4,iVar1);
    iVar3 = (**(code **)(*param_2 + 500))();
    iVar1 = FUN_0055c700(iVar3,iVar1,param_3);
    if ((iVar1 != 0) && (bVar4)) {
      return true;
    }
    bVar4 = false;
  }
  return bVar4;
}

