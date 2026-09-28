// FUN_0047b9e0

void __fastcall FUN_0047b9e0(void *param_1)

{
  void *this;
  uint *puVar1;
  void *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_006337a8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_10 = param_1;
  this = (void *)thunk_FUN_005f5060((int)param_1 + 0x44);
  puVar1 = FUN_00403040(this,&local_10);
  local_4 = 0;
  FUN_0047b8e0(param_1,puVar1);
  local_4 = 0xffffffff;
  FUN_00619730();
  ExceptionList = local_c;
  return;
}

