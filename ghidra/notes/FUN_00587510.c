
bool __thiscall FUN_00587510(void *this,int *param_1,undefined4 *param_2)

{
  int iVar1;
  
  *param_2 = 0;
  if (param_1 != (int *)0x0) {
    iVar1 = (**(code **)(*param_1 + 0x1e0))();
    *(int *)((int)this + 8) = *(int *)((int)this + 8) + iVar1;
  }
  return param_1 != (int *)0x0;
}

