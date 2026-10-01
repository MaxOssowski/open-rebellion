
undefined4 * __fastcall FUN_0057ddc0(undefined4 *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0064e1e8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0054f2c0(param_1);
  local_4 = 0;
  FUN_00583e80(param_1 + 0x14);
  *param_1 = &PTR_FUN_00669a30;
  ExceptionList = local_c;
  return param_1;
}

