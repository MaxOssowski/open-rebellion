
void __fastcall FUN_0041c230(int param_1)

{
  int iVar1;
  int iVar2;
  
  iVar2 = 1;
  for (iVar1 = FUN_0041c210(param_1); iVar1 != 0; iVar1 = *(int *)(iVar1 + 8)) {
    *(int *)(iVar1 + 0x14) = iVar2;
    iVar2 = iVar2 + 1;
  }
  return;
}

