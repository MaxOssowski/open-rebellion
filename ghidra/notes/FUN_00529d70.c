
void __thiscall FUN_00529d70(void *this,uint *param_1,undefined4 param_2,void *param_3)

{
  uint uVar1;
  int iVar2;
  int iVar3;
  
  uVar1 = FUN_0052a710(this,param_1,param_3);
  if (uVar1 != 0) {
    iVar2 = FUN_0053e140(*(int *)((int)this + 0x68),*(int *)((int)this + 0x60));
    iVar3 = FUN_005294e0(this,iVar2,param_3);
    if (iVar3 != 0) {
      iVar2 = FUN_00529540(this,*(int *)((int)this + 0x60) - iVar2,param_3);
      if (iVar2 != 0) {
        FUN_00529dd0(this,param_3);
      }
    }
  }
  return;
}

