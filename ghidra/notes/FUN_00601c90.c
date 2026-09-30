
void __thiscall FUN_00601c90(void *this,int param_1,HWND param_2)

{
  RECT local_10;
  
  if (*(int *)((int)this + 0xc) != param_1) {
    *(int *)((int)this + 0xc) = param_1;
    if (param_2 != (HWND)0x0) {
      local_10.left = *(LONG *)((int)this + 0x14);
      local_10.top = *(LONG *)((int)this + 0x18);
      local_10.right = *(LONG *)((int)this + 0x1c);
      local_10.bottom = *(LONG *)((int)this + 0x20);
      InvalidateRect(param_2,&local_10,0);
    }
  }
  return;
}

