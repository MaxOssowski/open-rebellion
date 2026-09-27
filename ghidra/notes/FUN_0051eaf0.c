
bool __fastcall FUN_0051eaf0(int param_1)

{
  int iVar1;
  int iVar2;
  
  iVar1 = *(int *)(param_1 + 0xb4);
  if (iVar1 != 0) {
    iVar2 = FUN_00583c40(iVar1);
    while (iVar2 != 0) {
      iVar2 = thunk_FUN_005f5e90(*(int *)(param_1 + 0xb4));
      FUN_004fd300(iVar2);
      iVar2 = FUN_00583c40(*(int *)(param_1 + 0xb4));
    }
  }
  return iVar1 != 0;
}

