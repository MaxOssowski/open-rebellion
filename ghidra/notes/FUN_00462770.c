
void __thiscall FUN_00462770(int *param_1,undefined4 param_2,uint param_3)

{
  uint uVar1;
  int iVar2;
  void *pvVar3;
  undefined4 *puVar4;
  char *pcVar5;
  short sVar6;
  undefined1 auStack_24 [4];
  int iStack_20;
  undefined4 auStack_1c [2];
  int iStack_14;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uVar1 = param_3;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_00631388;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  switch(param_3 & 0xffff) {
  case 100:
    ExceptionList = &pvStack_c;
    FUN_00462be0(param_1,param_3 >> 0x10);
    iVar2 = FUN_00609650((void *)param_1[0x55],param_1[0x54] + 0x98,0,0);
    if (iVar2 != 0) {
      puVar4 = &param_3;
      pvVar3 = (void *)FUN_0060a860((void *)param_1[0x5a],iVar2);
      puVar4 = FUN_0042d170(pvVar3,puVar4);
      uStack_4 = 0;
      FUN_004f26d0(param_1 + 0x5b,puVar4);
      uStack_4 = 0xffffffff;
      FUN_00619730();
      ExceptionList = pvStack_c;
      return;
    }
    break;
  case 0xc9:
    ExceptionList = &pvStack_c;
    FUN_00429440((void *)param_1[0x56],(uint *)(param_1 + 0x5b));
  case 200:
    (**(code **)(*param_1 + 0x30))();
    break;
  case 0xcc:
    ExceptionList = &pvStack_c;
    FUN_0060a790(auStack_1c,1);
    uStack_4 = 1;
    FUN_00609410((void *)param_1[0x55],auStack_1c);
    iStack_20 = iStack_14;
    if (iStack_14 == 0) {
      uStack_4 = 0xffffffff;
      FUN_0060a810(auStack_1c);
      ExceptionList = pvStack_c;
      return;
    }
    pvVar3 = (void *)FUN_0060a860((void *)param_1[0x5a],*(int *)(iStack_14 + 0xc));
    puVar4 = FUN_0042d170(pvVar3,auStack_24);
    uStack_4._0_1_ = 2;
    FUN_004f26d0(param_1 + 0x5b,puVar4);
    uStack_4 = CONCAT31(uStack_4._1_3_,1);
    FUN_00619730();
    sVar6 = (short)(uVar1 >> 0x10);
    if (sVar6 == 0x309) {
      FUN_00429440((void *)param_1[0x56],(uint *)(param_1 + 0x5b));
      (**(code **)(*param_1 + 0x30))();
    }
    else if (sVar6 == 0x29b) {
      pcVar5 = (char *)FUN_00583c40(iStack_20 + 0x14);
      FUN_00604f90((void *)param_1[0x54],pcVar5);
    }
    uStack_4 = 0xffffffff;
    FUN_0060a810(auStack_1c);
    ExceptionList = pvStack_c;
    return;
  case 0xfa:
    ExceptionList = &pvStack_c;
    FUN_004632d0(param_1,param_3 >> 0x10);
    ExceptionList = pvStack_c;
    return;
  }
  ExceptionList = pvStack_c;
  return;
}

