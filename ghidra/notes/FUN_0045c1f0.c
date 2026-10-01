
void FUN_0045c1f0(int *param_1,int param_2)

{
  int iVar1;
  int iVar2;
  int iVar3;
  
  iVar3 = *(int *)(param_2 + 0x28);
  iVar2 = *(int *)(param_2 + 0x2c);
  if (*(int **)(param_2 + 0x24) == (int *)0x0) {
    iVar3 = iVar3 + 0x14;
  }
  else {
    iVar1 = FUN_005fc0e0(*(int **)(param_2 + 0x24));
    iVar3 = iVar3 + iVar1 / 2;
  }
  iVar2 = iVar2 + *(int *)(param_2 + 0x34);
  *param_1 = iVar3 + -0x32;
  param_1[1] = iVar2;
  param_1[2] = iVar3 + 0x32;
  param_1[3] = iVar2 + 0x1e;
  return;
}

