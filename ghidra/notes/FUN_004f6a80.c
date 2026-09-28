
int __thiscall FUN_004f6a80(int *param_1,uint *param_2)

{
  bool bVar1;
  uint *puVar2;
  uint uVar3;
  undefined3 extraout_var;
  int iVar4;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0063fcd8;
  pvStack_c = ExceptionList;
  iVar4 = 1;
  if ((int *)param_1[7] == (int *)0x0) {
    ExceptionList = &pvStack_c;
    FUN_004ece80(param_2);
  }
  else {
    ExceptionList = &pvStack_c;
    iVar4 = (**(code **)(*(int *)param_1[7] + 0xc))(param_2);
  }
  if ((iVar4 != 0) && (puVar2 = FUN_004ece40(param_2), puVar2 == (uint *)0x0)) {
    if (((*(byte *)(param_1 + 0x14) & 0x40) != 0) && ((param_1[9] & 0x30U) == 0)) {
      uStack_4 = 0;
      uVar3 = (**(code **)(*param_1 + 4))();
      if ((uVar3 < 0xf2) || (0xf2 < uVar3)) {
        bVar1 = false;
      }
      else {
        bVar1 = true;
      }
      uStack_4 = 0xffffffff;
      FUN_00619730();
      if ((!bVar1) && (bVar1 = FUN_005406d0(param_1), CONCAT31(extraout_var,bVar1) != 0)) {
        ExceptionList = pvStack_c;
        return 0;
      }
    }
    iVar4 = 1;
  }
  ExceptionList = pvStack_c;
  return iVar4;
}

