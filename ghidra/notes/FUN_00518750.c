
bool __cdecl FUN_00518750(int param_1,int *param_2)

{
  uint uVar1;
  int iVar2;
  bool bVar3;
  
  bVar3 = true;
  *param_2 = 0;
  uVar1 = *(uint *)(param_1 + 0x24) >> 6 & 3;
  if (uVar1 == 1) {
    iVar2 = 1;
  }
  else if (uVar1 == 2) {
    iVar2 = 2;
  }
  else {
    iVar2 = 0;
  }
  if (iVar2 != 0) {
    iVar2 = FUN_00506f30(iVar2);
    bVar3 = iVar2 != 0;
    *param_2 = iVar2;
  }
  return bVar3;
}

