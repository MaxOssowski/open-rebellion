
undefined4 __cdecl
FUN_0055e4d0(undefined4 param_1,int param_2,int param_3,undefined4 *param_4,int *param_5)

{
  bool bVar1;
  undefined3 extraout_var;
  
  *param_4 = 0;
  *param_5 = 0;
  if ((param_2 != 0) && (param_3 != 0)) {
    *param_5 = param_3 + param_2 + DAT_006bb718;
  }
  if (0 < *param_5) {
    bVar1 = FUN_0053e2f0(*param_5);
    *param_4 = CONCAT31(extraout_var,bVar1);
  }
  return 1;
}

