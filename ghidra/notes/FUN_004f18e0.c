
undefined4 __thiscall FUN_004f18e0(int *param_1,undefined4 param_2,undefined4 param_3,void *param_4)

{
  void *pvVar1;
  bool bVar2;
  int iVar3;
  undefined4 *puVar4;
  undefined3 extraout_var;
  uint uVar5;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  pvVar1 = param_4;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0063f018;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar3 = FUN_004eef70(param_1,param_4);
  puVar4 = (undefined4 *)FUN_004ece30(&param_4);
  uStack_4 = 0;
  bVar2 = FUN_004ef190(param_1,puVar4);
  if ((CONCAT31(extraout_var,bVar2) == 0) || (iVar3 == 0)) {
    bVar2 = false;
  }
  else {
    bVar2 = true;
  }
  uStack_4 = 0xffffffff;
  FUN_00619730();
  iVar3 = FUN_004f78e0(param_1,1,pvVar1);
  if ((iVar3 == 0) || (!bVar2)) {
    bVar2 = false;
  }
  else {
    bVar2 = true;
  }
  iVar3 = FUN_004ece60((uint *)(param_1 + 0x1a));
  if (iVar3 != 0) {
    iVar3 = FUN_005345d0(param_1,1,pvVar1);
    if ((iVar3 == 0) || (!bVar2)) {
      bVar2 = false;
    }
    else {
      bVar2 = true;
    }
  }
  uVar5 = FUN_0053f950(0x30a,param_1,param_2,param_3,pvVar1);
  if ((uVar5 != 0) && (bVar2)) {
    ExceptionList = pvStack_c;
    return 1;
  }
  ExceptionList = pvStack_c;
  return 0;
}

