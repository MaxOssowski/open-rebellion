
bool __thiscall
FUN_00574630(void *param_1,int *param_2,int param_3,undefined4 *param_4,void *param_5)

{
  bool bVar1;
  undefined3 extraout_var;
  int iVar2;
  bool bVar3;
  
  bVar3 = true;
  *param_4 = 0;
  bVar1 = FUN_0053e2f0(param_3);
  if (CONCAT31(extraout_var,bVar1) != 0) {
    *param_4 = 1;
    iVar2 = FUN_00521880(param_1,3,param_5);
    bVar3 = iVar2 != 0;
    iVar2 = (**(code **)(*param_2 + 0x1d8))();
    if (iVar2 != 0) {
      iVar2 = FUN_00533ea0(param_2,*(short *)((int)param_2 + 0x5a) + DAT_006bb5c0,param_5);
      if ((iVar2 == 0) || (!bVar3)) {
        bVar1 = false;
      }
      else {
        bVar1 = true;
      }
      iVar2 = FUN_005340a0(param_2,*(short *)((int)param_2 + 0x62) + DAT_006bb57c,param_5);
      if ((iVar2 != 0) && (bVar1)) {
        return true;
      }
      bVar3 = false;
    }
  }
  return bVar3;
}

