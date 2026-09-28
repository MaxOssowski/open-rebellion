
int __thiscall FUN_0056e650(void *param_1,int *param_2,int param_3,int *param_4,void *param_5)

{
  int *this;
  undefined4 *puVar1;
  void *pvVar2;
  bool bVar3;
  bool bVar4;
  undefined3 extraout_var;
  uint *puVar5;
  undefined3 extraout_var_00;
  int iVar6;
  int *piVar7;
  int iVar8;
  undefined4 auStack_2c [7];
  int iStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  puVar1 = param_4;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0064bdc8;
  pvStack_c = ExceptionList;
  iVar8 = 1;
  ExceptionList = &pvStack_c;
  *param_4 = 0;
  bVar3 = FUN_0053e2f0(param_3);
  pvVar2 = param_5;
  if (CONCAT31(extraout_var,bVar3) != 0) {
    *puVar1 = 1;
    iVar8 = FUN_00521880(param_1,3,param_5);
    this = param_2;
    bVar3 = iVar8 != 0;
    iVar8 = (**(code **)(*param_2 + 0x1d8))();
    if (iVar8 != 0) {
      iVar8 = FUN_005340a0(this,*(short *)((int)this + 0x62) + DAT_006bb5a8,pvVar2);
      if ((iVar8 == 0) || (!bVar3)) {
        bVar3 = false;
      }
      else {
        bVar3 = true;
      }
      iVar8 = FUN_00533ea0(this,*(short *)((int)this + 0x5a) + DAT_006bb570,pvVar2);
      if ((iVar8 == 0) || (!bVar3)) {
        bVar3 = false;
      }
      else {
        bVar3 = true;
      }
    }
    param_4 = (int *)0x0;
    iVar8 = FUN_00586c80(param_1,(int *)&param_4);
    if ((iVar8 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    if (param_4 != (int *)0x0) {
      puVar5 = FUN_004025b0(this,(uint *)&param_2);
      uStack_4 = 0;
      iVar8 = (**(code **)(*param_4 + 0x210))(puVar5,pvVar2);
      if ((iVar8 == 0) || (!bVar3)) {
        bVar3 = false;
      }
      else {
        bVar3 = true;
      }
      uStack_4 = 0xffffffff;
      FUN_00619730();
    }
    param_3 = 0;
    bVar4 = FUN_00521050(param_1,&param_3);
    if ((CONCAT31(extraout_var_00,bVar4) == 0) || (!bVar3)) {
      iVar8 = 0;
    }
    else {
      iVar8 = 1;
    }
    if (param_3 != 0) {
      FUN_004f25c0(auStack_2c,param_3,1);
      uStack_4 = 1;
      FUN_00513120((int)auStack_2c);
      while (iStack_10 != 0) {
        iVar6 = FUN_0052bed0((int)auStack_2c);
        if ((*(uint *)(iVar6 + 0xac) & 1) != 0) {
          puVar5 = FUN_004025b0(this,(uint *)&param_2);
          uStack_4._0_1_ = 2;
          piVar7 = (int *)FUN_0052bed0((int)auStack_2c);
          iVar6 = (**(code **)(*piVar7 + 0x210))(puVar5,param_5);
          if ((iVar6 == 0) || (iVar8 == 0)) {
            iVar8 = 0;
          }
          else {
            iVar8 = 1;
          }
          uStack_4 = CONCAT31(uStack_4._1_3_,1);
          FUN_00619730();
        }
        FUN_005130d0((int)auStack_2c);
      }
      uStack_4 = 0xffffffff;
      FUN_004f26c0(auStack_2c);
    }
  }
  ExceptionList = pvStack_c;
  return iVar8;
}

