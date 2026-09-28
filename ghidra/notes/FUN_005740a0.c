
bool __thiscall
FUN_005740a0(void *param_1,int *param_2,int param_3,undefined4 *param_4,void *param_5)

{
  bool bVar1;
  undefined3 extraout_var;
  int iVar2;
  bool bVar3;
  
  bVar3 = true;
  bVar1 = FUN_0053e2f0(param_3);
  if (CONCAT31(extraout_var,bVar1) != 0) {
    *param_4 = 1;
    iVar2 = FUN_00521880(param_1,3,param_5);
    bVar3 = iVar2 != 0;
    iVar2 = (**(code **)(*param_2 + 0x1d8))();
    if (iVar2 != 0) {
      iVar2 = FUN_00533e20(param_2,(short)param_2[0x16] + DAT_006bb588,param_5);
      if ((iVar2 != 0) && (bVar3)) {
        return true;
      }
      bVar3 = false;
    }
  }
  return bVar3;
}

