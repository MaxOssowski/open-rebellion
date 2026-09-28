
void FUN_00530950(int *param_1,void *param_2)

{
  int *piVar1;
  void *pvVar2;
  int iVar3;
  uint uVar4;
  
  pvVar2 = param_2;
  piVar1 = param_1;
  iVar3 = 0;
  if (((uint)param_1[0x18] >> 2 & 1) != 0) {
    iVar3 = FUN_0053a860(param_1,0,param_2);
  }
  if ((iVar3 != 0) &&
     (uVar4 = FUN_0052eb60(piVar1,(int *)&param_1), uVar4 != 0 && param_1 != (int *)0x0)) {
    FUN_0052a430(param_1,pvVar2);
  }
  return;
}

