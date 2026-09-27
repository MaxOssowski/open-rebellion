
uint __thiscall FUN_0051d910(void *this,int *param_1)

{
  undefined4 uVar1;
  uint uVar2;
  
  if (param_1 != (int *)0x0) {
    uVar1 = (**(code **)(*param_1 + 0x10))();
    switch(uVar1) {
    case 1:
      uVar2 = FUN_0051e670(this,param_1);
      return uVar2;
    case 2:
      uVar2 = FUN_0051e6a0(this,param_1);
      return uVar2;
    case 3:
      uVar2 = FUN_0051e770(this,param_1);
      return uVar2;
    case 4:
      uVar2 = FUN_0051e840(this,param_1);
      return uVar2;
    }
  }
  return 0;
}

