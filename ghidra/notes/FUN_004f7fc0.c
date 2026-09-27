
uint __thiscall FUN_004f7fc0(void *this,void *param_1)

{
  int iVar1;
  int iVar2;
  
  if ((*(uint *)((int)this + 0x50) >> 0xb & 1) != 0) {
    iVar1 = FUN_004f7870(this,0,param_1);
    iVar2 = FUN_004f7390(this,0,param_1);
    if ((iVar2 != 0) && (iVar1 != 0)) {
      return 1;
    }
  }
  return 0;
}

