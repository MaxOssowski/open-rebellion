// FUN_0041be80

void * __thiscall FUN_0041be80(void *this,undefined4 param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0062b638;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_005f59a0(this);
  local_4 = 0;
  FUN_005f5b80((undefined4 *)((int)this + 0xc));
  *(undefined4 *)((int)this + 0xc) = &PTR_LAB_00658a68;
  *(undefined4 *)((int)this + 0x1c) = param_1;
  *(undefined ***)this = &PTR_FUN_00658a58;
  ExceptionList = local_c;
  return this;
}

