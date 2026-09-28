
bool __thiscall
FUN_00569c20(void *param_1,int *param_2,int param_3,undefined4 *param_4,void *param_5)

{
  bool bVar1;
  int iVar2;
  uint uVar3;
  undefined3 extraout_var;
  undefined3 extraout_var_00;
  bool bVar4;
  void *pvStack_4;
  
  pvStack_4 = (void *)0x0;
  iVar2 = FUN_00586720(param_1,(int *)&pvStack_4);
  if (pvStack_4 == (void *)0x0) {
    return iVar2 != 0;
  }
  uVar3 = *(uint *)((int)pvStack_4 + 0x88) & 4;
  if ((uVar3 == 0) || (iVar2 == 0)) {
    bVar4 = false;
  }
  else {
    bVar4 = true;
  }
  if (uVar3 == 0) {
    return bVar4;
  }
  bVar1 = FUN_0053e2f0(param_3);
  if (CONCAT31(extraout_var,bVar1) == 0) {
    return bVar4;
  }
  *param_4 = 1;
  iVar2 = FUN_0055cb10(*(uint *)((int)param_1 + 0x24) >> 6 & 3,
                       *(uint *)((int)pvStack_4 + 0x24) >> 6 & 3);
  bVar1 = FUN_0050c9f0(pvStack_4,iVar2,*(uint *)((int)param_1 + 0x24) >> 6 & 3,param_5);
  if ((CONCAT31(extraout_var_00,bVar1) == 0) || (bVar4 == false)) {
    bVar4 = false;
  }
  else {
    bVar4 = true;
  }
  if ((*(byte *)((int)pvStack_4 + 0x88) & 4) != 0) {
    iVar2 = FUN_0050c910(pvStack_4,param_5);
    if ((iVar2 == 0) || (!bVar4)) {
      bVar4 = false;
    }
    else {
      bVar4 = true;
    }
  }
  if ((*(byte *)((int)pvStack_4 + 0x88) & 4) == 0) {
    iVar2 = FUN_00521880(param_1,3,param_5);
    if ((iVar2 != 0) && (bVar4)) {
      bVar4 = true;
      goto LAB_00569d39;
    }
  }
  else {
    iVar2 = FUN_00521880(param_1,2,param_5);
    if ((iVar2 != 0) && (bVar4)) {
      bVar4 = true;
      goto LAB_00569d39;
    }
  }
  bVar4 = false;
LAB_00569d39:
  if ((*(int *)((int)param_1 + 0x60) == 3) &&
     (iVar2 = (**(code **)(*param_2 + 0x1d8))(), iVar2 != 0)) {
    iVar2 = FUN_00534120(param_2,(short)param_2[0x19] + DAT_006bb538,param_5);
    if ((iVar2 != 0) && (bVar4 != false)) {
      return true;
    }
    bVar4 = false;
  }
  return bVar4;
}

