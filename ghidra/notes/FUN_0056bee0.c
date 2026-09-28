
bool __thiscall FUN_0056bee0(void *param_1,void *param_2)

{
  bool bVar1;
  bool bVar2;
  int iVar3;
  undefined3 extraout_var;
  uint uVar4;
  undefined3 extraout_var_00;
  int *this;
  bool bVar5;
  int *piStack_18;
  uint uStack_14;
  uint uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0064b9f8;
  pvStack_c = ExceptionList;
  bVar2 = false;
  bVar5 = true;
  ExceptionList = &pvStack_c;
  if (*(int *)((int)param_1 + 0x60) == 0) {
    ExceptionList = &pvStack_c;
    iVar3 = FUN_00521880(param_1,2,param_2);
    bVar5 = iVar3 != 0;
  }
  if (*(int *)((int)param_1 + 0x60) == 3) {
    piStack_18 = (int *)0x0;
    bVar1 = FUN_00521070(param_1,(int *)&piStack_18);
    if ((CONCAT31(extraout_var,bVar1) == 0) || (bVar5 == false)) {
      bVar5 = false;
    }
    else {
      bVar5 = true;
    }
    this = (int *)0x0;
    if (piStack_18 != (int *)0x0) {
      uStack_14 = 0x90;
      uStack_10 = 0x98;
      uStack_4 = 0;
      uVar4 = (**(code **)(*piStack_18 + 4))();
      if ((uStack_14 <= uVar4) && (uVar4 < uStack_10)) {
        bVar2 = true;
      }
      uStack_4 = 0xffffffff;
      FUN_00619730();
      if ((bVar2) && (bVar5 != false)) {
        bVar5 = true;
      }
      else {
        bVar5 = false;
      }
      if (bVar2) {
        this = piStack_18;
      }
    }
    if (this != (int *)0x0) {
      bVar2 = FUN_0050d5a0(this,8,*(uint *)((int)param_1 + 0x24) >> 6 & 3,(int)param_2);
      if ((CONCAT31(extraout_var_00,bVar2) == 0) || (bVar5 == false)) {
        bVar5 = false;
      }
      else {
        bVar5 = true;
      }
    }
  }
  ExceptionList = pvStack_c;
  return bVar5;
}

