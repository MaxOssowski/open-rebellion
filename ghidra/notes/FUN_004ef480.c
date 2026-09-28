
bool __thiscall FUN_004ef480(int *param_1,undefined4 param_2,void *param_3)

{
  bool bVar1;
  int iVar2;
  undefined3 extraout_var;
  bool bVar3;
  undefined4 *unaff_retaddr;
  
  iVar2 = FUN_00524fb0();
  bVar3 = iVar2 != 0;
  if ((*(byte *)(param_1 + 0x14) & 8) == 0) {
    iVar2 = (**(code **)(*param_1 + 0x2e4))(param_3);
    if ((iVar2 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    if ((*(byte *)(param_1 + 0x14) & 8) == 0) {
      iVar2 = FUN_004ee3e0(param_1,0,param_3);
      if ((iVar2 == 0) || (bVar3 == false)) {
        bVar3 = false;
      }
      else {
        bVar3 = true;
      }
      bVar1 = FUN_004ef190(param_1,unaff_retaddr);
      if ((CONCAT31(extraout_var,bVar1) != 0) && (bVar3)) {
        return true;
      }
      bVar3 = false;
    }
  }
  return bVar3;
}

