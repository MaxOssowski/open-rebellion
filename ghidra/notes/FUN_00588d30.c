
bool __thiscall FUN_00588d30(void *this,int *param_1,int *param_2,void *param_3)

{
  int iVar1;
  uint uVar2;
  bool bVar3;
  
  iVar1 = thunk_FUN_00506e80();
  bVar3 = iVar1 != 0;
  if (iVar1 != 0) {
    uVar2 = FUN_00560200(param_1,param_2,param_3);
    if ((uVar2 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
  }
  if ((param_1[0x1e] & 0xcU) != 0) {
    if ((param_1[0x1e] & 1U) != 0) {
      *(int *)((int)this + 0x30) = *(int *)((int)this + 0x30) + -1;
      return bVar3;
    }
    *(int *)((int)this + 0x2c) = *(int *)((int)this + 0x2c) + -1;
  }
  return bVar3;
}

