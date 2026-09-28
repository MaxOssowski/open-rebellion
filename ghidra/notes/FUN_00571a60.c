
bool __thiscall
FUN_00571a60(void *param_1,int *param_2,int param_3,undefined4 *param_4,void *param_5)

{
  bool bVar1;
  int iVar2;
  undefined3 extraout_var;
  uint uVar3;
  bool bVar4;
  void *pvStack_4;
  
  pvStack_4 = (void *)0x0;
  iVar2 = FUN_00586720(param_1,(int *)&pvStack_4);
  bVar4 = iVar2 != 0;
  if (pvStack_4 == (void *)0x0) {
    return bVar4;
  }
  bVar1 = FUN_0053e2f0(param_3);
  if (CONCAT31(extraout_var,bVar1) == 0) {
    return bVar4;
  }
  *param_4 = 1;
  iVar2 = FUN_0050d030(pvStack_4,param_5);
  if ((iVar2 == 0) || (!bVar4)) {
    bVar4 = false;
  }
  else {
    bVar4 = true;
  }
  uVar3 = *(uint *)((int)param_1 + 0x24) >> 6 & 3;
  if (uVar3 == 3) {
    uVar3 = 3;
  }
  else {
    uVar3 = 2 - (uVar3 != 1);
  }
  if (((*(uint *)((int)pvStack_4 + 0x84) >> 2 & 3) == uVar3) ||
     (iVar2 = FUN_00509020(pvStack_4,*(uint *)((int)pvStack_4 + 0x24) >> 6 & 3,1), iVar2 != 0)) {
    iVar2 = FUN_00521880(param_1,2,param_5);
    if ((iVar2 != 0) && (bVar4)) {
      bVar4 = true;
      goto LAB_00571b44;
    }
  }
  else {
    iVar2 = FUN_00521880(param_1,3,param_5);
    if ((iVar2 != 0) && (bVar4)) {
      bVar4 = true;
      goto LAB_00571b44;
    }
  }
  bVar4 = false;
LAB_00571b44:
  if ((*(int *)((int)param_1 + 0x60) == 3) &&
     (iVar2 = (**(code **)(*param_2 + 0x1d8))(), iVar2 != 0)) {
    iVar2 = FUN_00534120(param_2,(short)param_2[0x19] + DAT_006bb5a0,param_5);
    if ((iVar2 != 0) && (bVar4 != false)) {
      return true;
    }
    bVar4 = false;
  }
  return bVar4;
}

