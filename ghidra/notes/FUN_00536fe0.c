
void __thiscall FUN_00536fe0(void *this,undefined4 *param_1)

{
  int iVar1;
  
  if (param_1 != (undefined4 *)0x0) {
    if (param_1[6] == 0) {
      (**(code **)*param_1)(1);
      return;
    }
    iVar1 = FUN_005f5500(this,param_1[6]);
    if (iVar1 != 0) {
      (**(code **)*param_1)(1);
      return;
    }
    FUN_005f5440(this,param_1);
  }
  return;
}

