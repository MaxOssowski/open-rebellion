
bool __thiscall FUN_0041c250(int param_1,int param_2,int param_3)

{
  bool bVar1;
  int iVar2;
  
  bVar1 = false;
  if ((param_2 != 0) && (param_3 != 0)) {
    if (*(int *)(param_2 + 0x10) < *(int *)(param_3 + 0x10)) {
      return *(int *)(param_1 + 0x1c) == 1;
    }
    if (*(int *)(param_3 + 0x10) < *(int *)(param_2 + 0x10)) {
      return *(int *)(param_1 + 0x1c) != 1;
    }
    iVar2 = FUN_0041cd80(10);
    bVar1 = iVar2 < 5;
  }
  return bVar1;
}

