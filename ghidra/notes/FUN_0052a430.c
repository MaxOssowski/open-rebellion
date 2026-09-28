
void __thiscall FUN_0052a430(void *this,void *param_1)

{
  int iVar1;
  
  if (*(int *)((int)this + 0x5c) < *(int *)((int)this + 0x68)) {
    iVar1 = FUN_005294e0(this,*(int *)((int)this + 0x5c) + 1,param_1);
  }
  else {
    iVar1 = FUN_00529540(this,*(int *)((int)this + 0x60) + 1,param_1);
  }
  if (iVar1 != 0) {
    FUN_00529dd0(this,param_1);
  }
  return;
}

