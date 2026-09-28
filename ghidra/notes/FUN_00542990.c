
bool FUN_00542990(int param_1,int param_2,undefined4 *param_3)

{
  int iVar1;
  undefined4 uVar2;
  bool bVar3;
  
  bVar3 = true;
  *param_3 = DAT_006b9050;
  if ((param_1 != 0) && (param_2 == 0)) {
    iVar1 = FUN_00506f50();
    bVar3 = iVar1 != 0;
    if (iVar1 != 0) {
      uVar2 = FUN_005725a0();
      *param_3 = uVar2;
    }
  }
  return bVar3;
}

