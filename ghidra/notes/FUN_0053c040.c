
undefined4 * __fastcall FUN_0053c040(undefined4 *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_00646378;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0051fa20(param_1);
  local_4 = 0;
  param_1[0x11] = 0;
  param_1[0x12] = 0;
  *param_1 = &PTR_FUN_006617f8;
  FUN_0053c710((int)param_1);
  ExceptionList = local_c;
  return param_1;
}

