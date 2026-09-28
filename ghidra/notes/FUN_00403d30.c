// FUN_00403d30

void * __fastcall FUN_00403d30(void *param_1)

{
  void *pvVar1;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  puStack_8 = &LAB_00629608;
  local_c = ExceptionList;
  local_4 = 0;
  ExceptionList = &local_c;
  pvVar1 = FUN_004f5940(param_1,(uint *)&stack0x00000004);
  local_4 = 0xffffffff;
  FUN_00619730();
  ExceptionList = local_c;
  return pvVar1;
}

