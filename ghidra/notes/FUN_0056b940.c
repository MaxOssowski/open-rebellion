// FUN_0056b940

undefined4 __thiscall FUN_0056b940(void *this,void *param_1)

{
  int *piVar1;
  void *local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0064b938;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  local_10 = this;
  piVar1 = (int *)FUN_004ece30(&local_10);
  local_4 = 0;
  FUN_0056b1a0(this,piVar1,param_1);
  local_4 = 0xffffffff;
  FUN_00619730();
  ExceptionList = local_c;
  return 1;
}

