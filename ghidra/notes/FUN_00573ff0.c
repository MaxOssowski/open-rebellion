
bool __thiscall FUN_00573ff0(void *param_1,int *param_2,undefined4 *param_3)

{
  int iVar1;
  uint uVar2;
  int iVar3;
  int iVar4;
  bool bVar5;
  void *pvStack_4;
  
  pvStack_4 = (void *)0x0;
  iVar1 = FUN_00586720(param_1,(int *)&pvStack_4);
  bVar5 = iVar1 != 0;
  if (pvStack_4 != (void *)0x0) {
    uVar2 = *(uint *)((int)param_1 + 0x24) >> 6 & 3;
    if (uVar2 == 3) {
      iVar1 = 3;
    }
    else {
      iVar1 = 2 - (uint)(uVar2 != 1);
    }
    iVar3 = FUN_005091f0(pvStack_4);
    iVar1 = FUN_00507270(pvStack_4,iVar1);
    iVar4 = (**(code **)(*param_2 + 0x1dc))();
    iVar1 = FUN_0055c680(iVar4,iVar1,iVar3,param_3);
    if ((iVar1 != 0) && (bVar5)) {
      return true;
    }
    bVar5 = false;
  }
  return bVar5;
}

