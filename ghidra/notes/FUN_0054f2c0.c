
undefined4 * __fastcall FUN_0054f2c0(undefined4 *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_00648738;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_0054eee0(param_1);
  local_4 = 0;
  FUN_00541e70(param_1 + 0x10);
  *param_1 = &PTR_FUN_00662088;
  ExceptionList = local_c;
  return param_1;
}

