
undefined4 __cdecl FUN_0053efd0(uint *param_1,int param_2)

{
  uint *this;
  undefined4 uVar1;
  int iVar2;
  
  this = param_1;
  if (DAT_006b90e4 != (void *)0x0) {
    this = FUN_00585dc0(DAT_006b90e4,param_1);
  }
  if (this == (uint *)0x0) {
    iVar2 = FUN_004ece60(param_1);
    if (((iVar2 != 0) && (DAT_006b90e8 != (void *)0x0)) && (*(int *)((int)DAT_006b90e8 + 4) != 0)) {
      FUN_004f5940(DAT_006b90e8,param_1);
    }
    return 0;
  }
  uVar1 = FUN_005844e0(this,param_2);
  return uVar1;
}

