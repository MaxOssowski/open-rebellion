
undefined4 __thiscall FUN_004424c0(int param_1,int param_2,LONG param_3,LONG param_4,int param_5)

{
  int iVar1;
  POINT pt;
  BOOL BVar2;
  int iVar3;
  tagRECT tStack_10;
  
  GetWindowRect(*(HWND *)(param_1 + 0x18),&tStack_10);
  MapWindowPoints(*(HWND *)(param_1 + 0x18),(HWND)0x0,(LPPOINT)&param_3,1);
  pt.y = param_4;
  pt.x = param_3;
  BVar2 = PtInRect(&tStack_10,pt);
  iVar1 = *(int *)(param_1 + 0x20);
  if (iVar1 != 0) {
    if (BVar2 == 0) {
      PostMessageA(*(HWND *)(iVar1 + 0x18),0x409,0,0);
      return 1;
    }
    if ((param_2 != 0) && (param_5 == 0x202)) {
      DAT_006b2900 = GetFocus();
      iVar3 = FUN_004ab550((undefined4 *)(param_2 + 0x78));
      if (iVar3 != 0) {
        if (*(int **)(param_1 + 0x150) != (int *)0x0) {
          (**(code **)(**(int **)(param_1 + 0x150) + 0x20))(iVar3,0,0);
        }
        PostMessageA(*(HWND *)(iVar1 + 0x18),0x409,0,0);
      }
    }
  }
  return 1;
}

