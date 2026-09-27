
void __thiscall FUN_0051e670(void *this,int *param_1)

{
  int iVar1;
  
  iVar1 = (**(code **)(*param_1 + 0x10))();
  if (iVar1 == 1) {
    param_1[3] = 0;
    FUN_0051eaa0(this,param_1);
  }
  return;
}

