
UINT FUN_0049f4b0(int *param_1,uint param_2,HPALETTE param_3,HWND param_4)

{
  UINT UVar1;
  
  if ((param_2 != 0x201) && (param_2 != 0x204)) {
    if (param_2 != 0x467) {
      UVar1 = FUN_004ac3a0(param_1,param_2,param_3,param_4);
      return UVar1;
    }
    InvalidateRect((HWND)param_1[6],(RECT *)(param_1 + 0x4a),0);
    return 0;
  }
  PostMessageA(*(HWND *)(param_1[8] + 0x18),0x467,(WPARAM)param_1[6],0);
  UVar1 = FUN_004ac3a0(param_1,param_2,param_3,param_4);
  return UVar1;
}

