
undefined4 __thiscall FUN_004fb810(void *this,undefined4 param_1,void *param_2)

{
  int iVar1;
  int iVar2;
  int unaff_retaddr;
  
  iVar1 = (**(code **)(*(int *)this + 0x84))(param_2);
  iVar2 = FUN_0053f9c0(0x300,this,unaff_retaddr,param_2);
  if ((iVar2 != 0) && (iVar1 != 0)) {
    return 1;
  }
  return 0;
}

