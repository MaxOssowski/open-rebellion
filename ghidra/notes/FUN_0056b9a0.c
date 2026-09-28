
int __thiscall
FUN_0056b9a0(void *param_1,int *param_2,void *param_3,undefined4 *param_4,void *param_5)

{
  undefined4 *puVar1;
  void *pvVar2;
  bool bVar3;
  undefined3 extraout_var;
  int iVar4;
  undefined3 extraout_var_00;
  uint *puVar5;
  int iVar6;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  puVar1 = param_4;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0064b958;
  pvStack_c = ExceptionList;
  iVar6 = 1;
  ExceptionList = &pvStack_c;
  *param_4 = 0;
  bVar3 = FUN_0053e2f0((int)param_3);
  if (CONCAT31(extraout_var,bVar3) != 0) {
    *puVar1 = 1;
    param_3 = (void *)0x0;
    iVar6 = FUN_00586720(param_1,(int *)&param_3);
    iVar4 = thunk_FUN_00506e80();
    if ((iVar4 == 0) || (iVar6 == 0)) {
      iVar6 = 0;
    }
    else {
      iVar6 = 1;
    }
    if ((param_3 != (void *)0x0) && (iVar4 != 0)) {
      FUN_004ece30(&param_4);
      pvVar2 = param_5;
      uStack_4 = 0;
      bVar3 = FUN_0055fc80(param_3,*(uint *)((int)param_1 + 0x24) >> 6 & 3,&param_4,param_5);
      if ((CONCAT31(extraout_var_00,bVar3) == 0) || (iVar6 == 0)) {
        iVar6 = 0;
      }
      else {
        iVar6 = 1;
      }
      puVar5 = FUN_004ece40((uint *)&param_4);
      if (puVar5 != (uint *)0x0) {
        iVar4 = FUN_0056b1a0(param_1,(int *)&param_4,pvVar2);
        if ((iVar4 == 0) || (iVar6 == 0)) {
          bVar3 = false;
        }
        else {
          bVar3 = true;
        }
        iVar6 = FUN_00521880(param_1,3,pvVar2);
        if ((iVar6 == 0) || (!bVar3)) {
          iVar6 = 0;
        }
        else {
          iVar6 = 1;
        }
        iVar4 = (**(code **)(*param_2 + 0x1d8))();
        if (iVar4 != 0) {
          iVar4 = FUN_00534120(param_2,(short)param_2[0x19] + DAT_006bb5b0,pvVar2);
          if ((iVar4 == 0) || (iVar6 == 0)) {
            iVar6 = 0;
          }
          else {
            iVar6 = 1;
          }
        }
      }
      uStack_4 = 0xffffffff;
      FUN_00619730();
    }
  }
  ExceptionList = pvStack_c;
  return iVar6;
}

