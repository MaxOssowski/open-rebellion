
int __thiscall FUN_00534c20(void *this,void *param_1)

{
  int iVar1;
  int iVar2;
  
  iVar2 = 1;
  if ((*(byte *)((int)this + 0x78) & 0x10) != 0) {
    iVar1 = FUN_004ece60((uint *)((int)this + 0x68));
    if (iVar1 != 0) {
      iVar2 = FUN_00534640(this,1,param_1);
    }
  }
  return iVar2;
}

