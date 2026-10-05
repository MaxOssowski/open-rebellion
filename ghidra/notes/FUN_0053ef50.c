
int * __cdecl FUN_0053ef50(uint *param_1,int param_2,int param_3)

{
  bool bVar1;
  int *piVar2;
  
  piVar2 = (int *)0x0;
  if ((param_2 < 1) || (2 < param_2)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  if (bVar1) {
    if (param_2 == 1) {
      piVar2 = FUN_0053f030(param_1,1,param_3);
      return piVar2;
    }
    piVar2 = FUN_0053f030(param_1,2,param_3);
  }
  return piVar2;
}

