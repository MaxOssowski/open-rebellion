
void __thiscall FUN_004a0560(int *param_1,uint *param_2,int *param_3)

{
  bool bVar1;
  int *this;
  uint *puVar2;
  int iVar3;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  puVar2 = param_2;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_00637568;
  pvStack_c = ExceptionList;
  this = param_3;
  ExceptionList = &pvStack_c;
  if (param_3 == (int *)0x0) {
    ExceptionList = &pvStack_c;
    this = FUN_004f2d10(*(int *)(param_1[0x53] + 0x9c),param_2);
  }
  if (this != (int *)0x0) {
    if ((*puVar2 >> 0x18 < 0x30) || (0x3f < *puVar2 >> 0x18)) {
      bVar1 = false;
    }
    else {
      bVar1 = true;
    }
    FUN_00619730();
    if (bVar1) {
      puVar2 = FUN_0042d170(this,&param_3);
      uStack_4 = 0;
      iVar3 = FUN_004ece60(puVar2);
      uStack_4 = 0xffffffff;
      FUN_00619730();
      if ((iVar3 != 0) && ((this[0x1e] & 0x100U) == 0)) {
        puVar2 = FUN_0042d170(this,&param_3);
        uStack_4 = 1;
        iVar3 = FUN_0060a860((void *)param_1[0x6d],*puVar2 & 0xffffff);
        uStack_4 = 0xffffffff;
        FUN_00619730();
        if (iVar3 == 0) {
          FUN_004a1590(param_1);
          puVar2 = FUN_0042d170(this,&param_2);
          uStack_4 = 2;
          iVar3 = FUN_0060a860((void *)param_1[0x6d],*puVar2 & 0xffffff);
          uStack_4 = 0xffffffff;
          FUN_00619730();
          if (iVar3 == 0) {
            ExceptionList = pvStack_c;
            return;
          }
        }
        if ((*(byte *)(iVar3 + 0x3c) & 1) != 0) {
          FUN_004a0c60(param_1,iVar3,1);
        }
      }
    }
  }
  ExceptionList = pvStack_c;
  return;
}

