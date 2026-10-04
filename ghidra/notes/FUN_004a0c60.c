
void __thiscall FUN_004a0c60(void *this,int param_1,int param_2)

{
  int iVar1;
  int iVar2;
  
  iVar2 = 0;
  iVar1 = 0;
  if (param_1 != 0) {
    iVar2 = *(int *)(param_1 + 0xc);
  }
  if (*(int *)((int)this + 0x1c4) != 0) {
    iVar1 = *(int *)(*(int *)((int)this + 0x1c4) + 0xc);
  }
  if ((iVar2 != iVar1) || (param_2 != 0)) {
    *(int *)((int)this + 0x1c4) = param_1;
    FUN_004a0ca0(this);
  }
  return;
}

