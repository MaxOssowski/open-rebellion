// FUN_00524b20

undefined4 __thiscall FUN_00524b20(void *this,undefined4 param_1,int param_2,void *param_3)

{
  bool bVar1;
  undefined4 uVar2;
  int iVar3;
  undefined3 extraout_var;
  
  uVar2 = 1;
  if (param_2 != 0) {
    iVar3 = FUN_005227d0(this,param_3);
    bVar1 = FUN_005229c0(this,param_3);
    if ((CONCAT31(extraout_var,bVar1) != 0) && (iVar3 != 0)) {
      return 1;
    }
    uVar2 = 0;
  }
  return uVar2;
}

