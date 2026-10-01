
int __thiscall FUN_004420b0(void *this,int param_1,int param_2)

{
  int iVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  void *this_00;
  int iVar5;
  int iVar6;
  
  this_00 = (void *)thunk_FUN_005f5060((int)this + 0x18);
  iVar1 = *(int *)((int)this + 0x50);
  iVar2 = *(int *)((int)this + 0x4c);
  iVar3 = *(int *)((int)this + 0x48);
  iVar6 = 0;
  iVar4 = *(int *)((int)this + 0x54);
  while ((this_00 != (void *)0x0 && (iVar6 == 0))) {
    iVar5 = FUN_00472b50(this_00,param_1 + (iVar3 - iVar1),param_2 + (iVar2 - iVar4));
    if (iVar5 != 0) {
      iVar6 = *(int *)((int)this_00 + 0x18);
    }
    this_00 = *(void **)((int)this_00 + 0x10);
  }
  return iVar6;
}

