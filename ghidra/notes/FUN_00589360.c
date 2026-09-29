
bool __thiscall FUN_00589360(int param_1,int *param_2,uint *param_3,void *param_4)

{
  bool bVar1;
  uint *puVar2;
  int iVar3;
  undefined3 extraout_var;
  undefined4 auStack_14 [2];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0064fa70;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  *param_3 = 0;
  puVar2 = FUN_004ece40((uint *)(param_2 + 0x27));
  if (puVar2 == (uint *)0x0) {
    FUN_00589190(auStack_14,*(undefined4 *)(param_1 + 4));
    uStack_4 = 0;
    iVar3 = FUN_005891e0(auStack_14,param_2,param_3,param_4);
    *param_3 = 0;
    uStack_4 = 0xffffffff;
    FUN_005891d0(auStack_14);
  }
  else {
    FUN_00589250(auStack_14,*(undefined4 *)(param_1 + 4));
    uStack_4 = 1;
    bVar1 = FUN_005892a0(auStack_14,param_2,param_3,param_4);
    iVar3 = CONCAT31(extraout_var,bVar1);
    uStack_4 = 0xffffffff;
    FUN_00589290(auStack_14);
  }
  ExceptionList = pvStack_c;
  return iVar3 != 0;
}

