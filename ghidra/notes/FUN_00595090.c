// FUN_00595090

int __thiscall FUN_00595090(void *this,int param_1)

{
  int iVar1;
  int iVar2;
  
  iVar1 = thunk_FUN_005f5060((int)this + 0x30);
  if (iVar1 != 0) {
    do {
      if (param_1 < *(int *)(iVar1 + 0x20)) break;
      iVar1 = *(int *)(iVar1 + 0x10);
    } while (iVar1 != 0);
    if (iVar1 != 0) {
      iVar2 = FUN_005f5c60(iVar1);
      if (iVar2 == 0) {
        return iVar1;
      }
      iVar1 = FUN_005f5c60(iVar1);
      return iVar1;
    }
  }
  iVar1 = thunk_FUN_005f5080((int)this + 0x30);
  return iVar1;
}

