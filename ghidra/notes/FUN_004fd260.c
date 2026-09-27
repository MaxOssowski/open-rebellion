
int FUN_004fd260(void)

{
  int iVar1;
  int iVar2;
  
  iVar2 = 0;
  iVar1 = FUN_0051ce00();
  if (iVar1 != 0) {
    iVar2 = *(int *)(iVar1 + 0x18) + 1;
    *(int *)(iVar1 + 0x18) = iVar2;
  }
  return iVar2;
}

