
bool __thiscall FUN_005890d0(int param_1,int *param_2,int *param_3,undefined4 param_4)

{
  int *piVar1;
  bool bVar2;
  undefined3 extraout_var;
  
  piVar1 = param_3;
  *param_3 = 0;
  param_3 = (int *)0x0;
  bVar2 = FUN_00588da0(*(void **)(param_1 + 4),param_2,(int *)&param_3);
  *piVar1 = (int)param_3;
  return CONCAT31(extraout_var,bVar2) != 0;
}

