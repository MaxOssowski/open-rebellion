
void __thiscall FUN_004ef150(void *this,void *param_1)

{
  int iVar1;
  
  iVar1 = 0;
  if ((*(uint *)((int)this + 0xac) >> 8 & 1) != 0) {
    iVar1 = FUN_004eebf0(this,1,param_1);
  }
  if (iVar1 != 0) {
    FUN_004eebf0(this,0,param_1);
  }
  return;
}

