
bool __thiscall FUN_0056e870(void *param_1,void *param_2)

{
  void *pvVar1;
  int iVar2;
  int *piVar3;
  void *this;
  bool bVar4;
  void *pvVar5;
  undefined4 auStack_3c [11];
  int iStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  uint uStack_4;
  
  pvVar1 = param_2;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0064bdf0;
  pvStack_c = ExceptionList;
  bVar4 = true;
  if (*(int *)((int)param_1 + 0x60) == 0) {
    ExceptionList = &pvStack_c;
    iVar2 = FUN_00521880(param_1,2,param_2);
    bVar4 = iVar2 != 0;
    FUN_00525fe0(auStack_3c,param_1);
    uStack_4 = 0;
    FUN_004ece30(&param_2);
    uStack_4 = CONCAT31(uStack_4._1_3_,1);
    FUN_00525930((int)auStack_3c);
    while (iStack_10 != 0) {
      piVar3 = (int *)FUN_00525f20((int)auStack_3c);
      iVar2 = (**(code **)(*piVar3 + 0x20c))(&param_2,pvVar1);
      if ((iVar2 == 0) || (bVar4 == false)) {
        bVar4 = false;
      }
      else {
        bVar4 = true;
      }
      iVar2 = 0;
      pvVar5 = pvVar1;
      this = (void *)FUN_00525f20((int)auStack_3c);
      iVar2 = FUN_004eeb10(this,iVar2,pvVar5);
      if ((iVar2 == 0) || (!bVar4)) {
        bVar4 = false;
      }
      else {
        bVar4 = true;
      }
      FUN_005258f0((int)auStack_3c);
    }
    uStack_4 = uStack_4 & 0xffffff00;
    FUN_00619730();
    uStack_4 = 0xffffffff;
    FUN_00526080(auStack_3c);
  }
  ExceptionList = pvStack_c;
  return bVar4;
}

