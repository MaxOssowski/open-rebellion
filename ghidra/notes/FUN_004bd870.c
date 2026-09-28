// FUN_004bd870

void __fastcall FUN_004bd870(undefined4 *param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_00639dc8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  *param_1 = &PTR_FUN_0065c408;
  local_4 = 0;
  FUN_00619730();
  local_4 = 0xffffffff;
  FUN_0047abd0(param_1);
  ExceptionList = local_c;
  return;
}

