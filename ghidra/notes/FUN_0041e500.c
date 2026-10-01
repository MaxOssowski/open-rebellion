
undefined4 FUN_0041e500(int *param_1)

{
  bool bVar1;
  undefined3 extraout_var;
  int iVar2;
  
  FUN_0041d030();
  bVar1 = FUN_0041e1a0();
  if (CONCAT31(extraout_var,bVar1) != 0) {
    iVar2 = FUN_0051ce00();
    if (*(int *)(iVar2 + 0x24) != 0) {
      FUN_0041d050();
    }
    iVar2 = FUN_0051ce00();
    iVar2 = FUN_0051dcb0(iVar2);
    *param_1 = iVar2;
    if (iVar2 == 0) {
      FUN_0041db60();
    }
  }
  return 1;
}

