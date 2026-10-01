
void __thiscall FUN_00606980(void *this,int param_1,int param_2,int param_3,int param_4)

{
  SetWindowPos(*(HWND *)((int)this + 0x18),(HWND)0x0,
               ((param_3 - *(int *)((int)this + 0x38)) - param_1) / 2 + param_1,
               ((param_4 - *(int *)((int)this + 0x3c)) - param_2) / 2 + param_2,0,0,5);
  return;
}

