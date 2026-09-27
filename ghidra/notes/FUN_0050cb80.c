
undefined4 __thiscall FUN_0050cb80(void *this,void *param_1)

{
  uint uVar1;
  bool bVar2;
  int iVar3;
  
  uVar1 = *(uint *)((int)this + 0x88);
  iVar3 = FUN_0050aa50(this,1,param_1);
  if ((iVar3 == 0) || ((uVar1 >> 2 & 1) == 0)) {
    bVar2 = false;
  }
  else {
    bVar2 = true;
  }
  iVar3 = FUN_0050aa50(this,0,param_1);
  if ((iVar3 != 0) && (bVar2)) {
    return 1;
  }
  return 0;
}

