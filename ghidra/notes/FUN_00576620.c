
bool __thiscall FUN_00576620(void *param_1,int *param_2,int param_3,int *param_4,void *param_5)

{
  undefined4 *puVar1;
  void *pvVar2;
  bool bVar3;
  undefined3 extraout_var;
  int iVar4;
  bool bVar5;
  
  puVar1 = param_4;
  bVar5 = true;
  *param_4 = 0;
  bVar3 = FUN_0053e2f0(param_3);
  if (CONCAT31(extraout_var,bVar3) != 0) {
    *puVar1 = 1;
    param_4 = (int *)0x0;
    iVar4 = FUN_00586c80(param_1,(int *)&param_4);
    pvVar2 = param_5;
    bVar5 = iVar4 != 0;
    if (param_4 != (int *)0x0) {
      iVar4 = (**(code **)(*param_4 + 0x2ec))(param_5);
      if ((iVar4 == 0) || (!bVar5)) {
        bVar5 = false;
      }
      else {
        bVar5 = true;
      }
    }
    if ((param_4 != (int *)0x0) && ((*(byte *)(param_4 + 0x14) & 8) != 0)) {
      iVar4 = FUN_00521880(param_1,3,pvVar2);
      if ((iVar4 == 0) || (bVar5 == false)) {
        bVar5 = false;
      }
      else {
        bVar5 = true;
      }
      iVar4 = (**(code **)(*param_2 + 0x1d8))();
      if (iVar4 != 0) {
        iVar4 = FUN_005340a0(param_2,*(short *)((int)param_2 + 0x62) + DAT_006bb584,pvVar2);
        if ((iVar4 != 0) && (bVar5 != false)) {
          return true;
        }
        bVar5 = false;
      }
    }
  }
  return bVar5;
}

