
undefined4 __fastcall FUN_0054ec90(void *param_1)

{
  int *piVar1;
  int iVar2;
  undefined4 uVar3;
  
  uVar3 = 0;
  piVar1 = (int *)FUN_0041c210((int)param_1);
  if (piVar1 != (int *)0x0) {
    FUN_005f6250(param_1,(int)piVar1);
    uVar3 = 1;
    iVar2 = (**(code **)(*piVar1 + 0x20))();
    if (iVar2 != 0) {
      FUN_0054ece0(param_1,(int)piVar1);
      return 1;
    }
    (**(code **)*piVar1)(1);
  }
  return uVar3;
}

