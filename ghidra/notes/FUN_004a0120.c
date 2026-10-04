
void __thiscall FUN_004a0120(int *param_1,undefined4 param_2,uint param_3)

{
  uint uVar1;
  int *this;
  int iVar2;
  int iVar3;
  undefined4 auStack_1c [4];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uVar1 = param_3;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_00637500;
  pvStack_c = ExceptionList;
  switch(param_3 & 0xffff) {
  case 10:
    ExceptionList = &pvStack_c;
    FUN_0060a790(auStack_1c,1);
    uStack_4 = 1;
    if (uVar1 >> 0x10 == 0x29b) {
      iVar3 = 0;
      iVar2 = FUN_004a2430((int)param_1);
      FUN_004a0c60(param_1,iVar2,iVar3);
    }
    PostMessageA(*(HWND *)(param_1[0x53] + 0x18),0x467,param_1[6],0);
    uStack_4 = 0xffffffff;
    FUN_0060a810(auStack_1c);
    ExceptionList = pvStack_c;
    return;
  case 0xb:
    ExceptionList = &pvStack_c;
    SetFocus(*(HWND *)(param_1[0x6e] + 0x18));
    PostMessageA(*(HWND *)(param_1[0x53] + 0x18),0x467,param_1[6],0);
    ExceptionList = pvStack_c;
    return;
  case 0x14:
    ExceptionList = &pvStack_c;
    (**(code **)(*param_1 + 0x30))();
    ExceptionList = pvStack_c;
    return;
  case 0x15:
    ExceptionList = &pvStack_c;
    PostMessageA(*(HWND *)(param_1[0x53] + 0x18),0x111,param_1[9] & 0xffffU | 0x4660000,0);
    ExceptionList = pvStack_c;
    return;
  case 0x16:
    ExceptionList = &pvStack_c;
    SendMessageA(*(HWND *)(param_1[0x53] + 0x18),0x467,param_1[6],0);
    FUN_004a1ba0(param_1,uVar1 >> 0x10,0);
    SetFocus(*(HWND *)(param_1[0x70] + 0x18));
    break;
  case 0xca:
    ExceptionList = &pvStack_c;
    this = FUN_004a25c0(param_1);
    if ((this != (int *)0x0) && (param_1[0x53] != 0)) {
      FUN_004ece30(&param_3);
      uStack_4 = 0;
      iVar2 = FUN_004f6b70(this,&param_3);
      if (iVar2 != 0) {
        FUN_00429ce0((void *)param_1[0x53],&param_3);
      }
      uStack_4 = 0xffffffff;
      FUN_00619730();
      ExceptionList = pvStack_c;
      return;
    }
  }
  ExceptionList = pvStack_c;
  return;
}

