
bool __fastcall FUN_00520d60(undefined4 param_1)

{
  bool bVar1;
  undefined4 local_3c [12];
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_00643708;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_00525bb0(local_3c,param_1);
  local_4 = 0;
  FUN_00525930((int)local_3c);
  bVar1 = FUN_00525a00((int)local_3c);
  local_4 = 0xffffffff;
  FUN_00525c50(local_3c);
  ExceptionList = local_c;
  return bVar1;
}

