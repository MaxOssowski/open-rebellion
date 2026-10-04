
void __thiscall FUN_004a0c20(void *this,int param_1)

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
  if (iVar2 == iVar1) {
    *(undefined4 *)((int)this + 0x1c4) = 0;
    FUN_004a0ca0(this);
  }
  return;
}

