
int __thiscall FUN_00500820(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  iVar1 = 0;
  if ((*(byte *)((int)this + 0x50) & 0x40) != 0) {
    iVar2 = (**(code **)(*(int *)this + 0x210))();
    iVar1 = (**(code **)(*(int *)this + 0x1ec))();
    iVar1 = iVar1 - iVar2;
    if (iVar1 == 0) {
      iVar2 = (**(code **)(*(int *)this + 0x214))();
      iVar1 = (**(code **)(*(int *)this + 0x1f0))();
      iVar1 = iVar1 - iVar2;
    }
    if ((iVar1 == 0) && ((char)((uint)*(undefined4 *)((int)this + 0x40) >> 8) == '\0')) {
      iVar1 = FUN_004f63f0(this,param_1);
    }
  }
  return iVar1;
}

