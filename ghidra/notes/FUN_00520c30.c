
undefined4 __thiscall FUN_00520c30(void *param_1,uint *param_2)

{
  bool bVar1;
  undefined3 extraout_var;
  undefined3 extraout_var_00;
  undefined3 extraout_var_01;
  
  bVar1 = FUN_00520bd0(param_1,param_2);
  if (CONCAT31(extraout_var,bVar1) == 0) {
    bVar1 = FUN_00520bf0(param_1,param_2);
    if (CONCAT31(extraout_var_00,bVar1) == 0) {
      bVar1 = FUN_00520c10(param_1,param_2);
      if (CONCAT31(extraout_var_01,bVar1) == 0) {
        return 0;
      }
    }
  }
  return 1;
}

