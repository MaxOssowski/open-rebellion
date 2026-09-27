
undefined4 __thiscall FUN_004fba10(void *this,uint param_1,void *param_2)

{
  uint uVar1;
  int iVar2;
  
  uVar1 = FUN_0053fa60(0x387,param_1,this,(int *)(*(int *)((int)this + 0x54) + 0x10),param_2);
  iVar2 = FUN_0053f9c0(0x306,this,param_1,param_2);
  if ((iVar2 != 0) && (uVar1 != 0)) {
    return 1;
  }
  return 0;
}

