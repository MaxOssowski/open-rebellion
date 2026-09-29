
void __thiscall FUN_005873c0(void *this,int param_1,uint *param_2)

{
  uint uVar1;
  
  *param_2 = 0;
  if (param_1 != 0) {
    uVar1 = (uint)(*(int *)((int)this + 8) == *(int *)((int)this + 0xc));
    *param_2 = uVar1;
    if (uVar1 != 0) {
      *(int *)((int)this + 0x10) = param_1;
      return;
    }
    *(int *)((int)this + 8) = *(int *)((int)this + 8) + 1;
  }
  return;
}

