
UINT FUN_00442d10(int *param_1,uint param_2,HPALETTE param_3,HWND param_4)

{
  UINT UVar1;
  
  if (param_2 != 0x405) {
    UVar1 = FUN_004aab50(param_1,param_2,param_3,param_4);
    return UVar1;
  }
  if ((HPALETTE)param_1[6] == param_3) {
    PostMessageA(*(HWND *)(param_1[8] + 0x18),0x467,0,0);
    FUN_00442430();
  }
  return 0;
}

