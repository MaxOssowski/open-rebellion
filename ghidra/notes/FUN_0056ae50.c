
int __thiscall FUN_0056ae50(void *param_1,int *param_2,uint param_3,int *param_4,void *param_5)

{
  undefined4 *puVar1;
  void *pvVar2;
  bool bVar3;
  undefined3 extraout_var;
  uint *puVar4;
  int iVar5;
  int iVar6;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  puVar1 = param_4;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0064b7b8;
  pvStack_c = ExceptionList;
  iVar6 = 1;
  ExceptionList = &pvStack_c;
  *param_4 = 0;
  bVar3 = FUN_0053e2f0(param_3);
  pvVar2 = param_5;
  if (CONCAT31(extraout_var,bVar3) != 0) {
    *puVar1 = 1;
    FUN_00521880(param_1,3,param_5);
    iVar6 = (**(code **)(*param_2 + 0x1d8))();
    if (iVar6 != 0) {
      FUN_005340a0(param_2,*(short *)((int)param_2 + 0x62) + DAT_006bb55c,pvVar2);
    }
    param_4 = (int *)0x0;
    iVar6 = FUN_00586c80(param_1,(int *)&param_4);
    if (param_4 != (int *)0x0) {
      puVar4 = FUN_004025b0(param_2,&param_3);
      uStack_4 = 0;
      iVar5 = (**(code **)(*param_4 + 0x210))(puVar4,pvVar2);
      if ((iVar5 == 0) || (iVar6 == 0)) {
        iVar6 = 0;
      }
      else {
        iVar6 = 1;
      }
      uStack_4 = 0xffffffff;
      FUN_00619730();
    }
  }
  ExceptionList = pvStack_c;
  return iVar6;
}

