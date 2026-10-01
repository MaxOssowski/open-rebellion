
undefined4 __thiscall FUN_00609fc0(void *this,LONG param_1,LONG param_2,int param_3)

{
  int *piVar1;
  HWND pHVar2;
  
  piVar1 = *(int **)((int)this + 0x20);
  pHVar2 = GetCapture();
  if (((pHVar2 == (HWND)0x0) && (piVar1 != (int *)0x0)) && (param_3 != 0)) {
    MapWindowPoints(*(HWND *)(param_3 + 0x18),*(HWND *)((int)this + 0x18),(LPPOINT)&param_1,1);
    (**(code **)(*piVar1 + 0x1c))(param_1,param_2,this);
  }
  return 0;
}

