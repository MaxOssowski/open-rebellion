// FUN_00534950

void __thiscall FUN_00534950(void *this,void *param_1)

{
  int iVar1;
  
  iVar1 = FUN_004ece60((uint *)((int)this + 0x68));
  if ((iVar1 == 0) && ((*(byte *)((int)this + 0x78) & 2) == 0)) {
    FUN_00534800(this,0,param_1);
    return;
  }
  FUN_00534800(this,1,param_1);
  return;
}

