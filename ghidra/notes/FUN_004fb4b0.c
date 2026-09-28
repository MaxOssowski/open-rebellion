
undefined4 __thiscall FUN_004fb4b0(void *this,undefined4 param_1,void *param_2)

{
  bool bVar1;
  int iVar2;
  int iVar3;
  int unaff_retaddr;
  
  iVar2 = FUN_004f7aa0(this,1,param_2);
  iVar3 = (**(code **)(*(int *)this + 0x84))(param_2);
  if ((iVar3 == 0) || (iVar2 == 0)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  iVar2 = FUN_0053f9c0(0x308,this,unaff_retaddr,param_2);
  if ((iVar2 != 0) && (bVar1)) {
    return 1;
  }
  return 0;
}

