
undefined4 * __fastcall FUN_00550620(undefined4 *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_00648a03;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_004ecc00(param_1);
  local_4 = 0;
  param_1[7] = 1;
  FUN_004ece30(param_1 + 8);
  local_4 = CONCAT31(local_4._1_3_,1);
  FUN_00550020(param_1 + 9);
  *param_1 = &PTR_FUN_006620f8;
  ExceptionList = local_c;
  return param_1;
}

