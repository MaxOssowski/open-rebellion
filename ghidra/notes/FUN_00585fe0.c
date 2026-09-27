
undefined4 * __fastcall FUN_00585fe0(undefined4 *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0064f608;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0054f1b0(param_1);
  local_4 = 0;
  FUN_005401b0(param_1 + 0x11);
  *param_1 = &PTR_FUN_0066a230;
  ExceptionList = local_c;
  return param_1;
}

