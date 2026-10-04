
void * __thiscall FUN_0060a790(void *this,undefined4 param_1)

{
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  local_4 = 0xffffffff;
  puStack_8 = &LAB_006568c8;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  FUN_005f59a0(this);
  local_4 = 0;
  FUN_005f4950((undefined4 *)((int)this + 0xc),0);
  *(undefined ***)this = &PTR_FUN_0066e148;
  *(undefined4 *)((int)this + 0xc) = param_1;
  ExceptionList = local_c;
  return this;
}

