
void __thiscall FUN_004a9890(int *param_1,undefined4 param_2,uint param_3,LPARAM param_4)

{
  HWND hWnd;
  uint uVar1;
  int *this;
  int iVar2;
  WPARAM wParam;
  UINT Msg;
  LPARAM lParam;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uVar1 = param_3;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_00638188;
  pvStack_c = ExceptionList;
  hWnd = *(HWND *)(param_1[0x53] + 0x18);
  switch(param_3 & 0xffff) {
  case 100:
    ExceptionList = &pvStack_c;
    SendMessageA(hWnd,0x467,param_1[6],0);
    FUN_004a90d0(param_1,uVar1 >> 0x10);
    SetFocus(*(HWND *)(param_1[0x59] + 0x18));
    ExceptionList = pvStack_c;
    return;
  case 200:
    ExceptionList = &pvStack_c;
    (**(code **)(*param_1 + 0x30))();
    ExceptionList = pvStack_c;
    return;
  case 0xc9:
    ExceptionList = &pvStack_c;
    PostMessageA(hWnd,0x111,param_1[9] & 0xffffU | 0x4660000,0);
    ExceptionList = pvStack_c;
    return;
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
    break;
  case 0xcb:
    if (param_3 >> 0x10 == 0x29a) {
      wParam = param_1[9] & 0xffffU | 0x29a0000;
      Msg = 0x111;
      lParam = param_4;
    }
    else {
      wParam = param_1[6];
      lParam = 0;
      Msg = 0x467;
    }
    ExceptionList = &pvStack_c;
    SendMessageA(hWnd,Msg,wParam,lParam);
  }
  ExceptionList = pvStack_c;
  return;
}

