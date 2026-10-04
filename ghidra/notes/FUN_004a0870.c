
void __thiscall FUN_004a0870(int *param_1,uint *param_2,int *param_3)

{
  bool bVar1;
  int *this;
  uint *puVar2;
  int iVar3;
  void *this_00;
  undefined *puVar4;
  char *pcVar5;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_006375a8;
  pvStack_c = ExceptionList;
  this = param_3;
  ExceptionList = &pvStack_c;
  if (param_3 == (int *)0x0) {
    ExceptionList = &pvStack_c;
    this = FUN_004f2d10(*(int *)(param_1[0x53] + 0x9c),param_2);
  }
  if (this != (int *)0x0) {
    if ((*param_2 >> 0x18 < 0x30) || (0x3f < *param_2 >> 0x18)) {
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
        this_00 = (void *)FUN_0060a860((void *)param_1[0x6f],this[6]);
        if (this_00 != (void *)0x0) {
          puVar2 = FUN_0042c3b0(param_1[0x6c],param_2,this,0,1);
          FUN_0060be60(this_00,(int)puVar2,0);
          puVar4 = FUN_004f62d0((int)this);
          pcVar5 = (char *)FUN_00583c40((int)puVar4);
          FUN_005f35e0((void *)((int)this_00 + 0x14),pcVar5);
          FUN_0060a280((void *)param_1[0x70]);
          FUN_004a0ca0(param_1);
        }
      }
    }
  }
  ExceptionList = pvStack_c;
  return;
}

