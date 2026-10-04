
UINT FUN_004a86d0(int *param_1,uint param_2,HPALETTE param_3,HWND param_4)

{
  UINT UVar1;
  
  if (param_2 < 0x202) {
    if (param_2 == 0x201) {
LAB_004a8748:
      SendMessageA(*(HWND *)(param_1[0x53] + 0x18),0x467,(WPARAM)param_1[6],0);
      SetFocus(*(HWND *)(param_1[0x59] + 0x18));
      FUN_004ac3a0(param_1,param_2,param_3,param_4);
      return 0;
    }
    if (param_2 == 7) {
      SetFocus(*(HWND *)(param_1[0x59] + 0x18));
      return 0;
    }
  }
  else {
    if (param_2 == 0x204) goto LAB_004a8748;
    if (param_2 == 0x467) {
      InvalidateRect((HWND)param_1[6],(RECT *)(param_1 + 0x4a),0);
      return 0;
    }
  }
  UVar1 = FUN_004ac3a0(param_1,param_2,param_3,param_4);
  return UVar1;
}

