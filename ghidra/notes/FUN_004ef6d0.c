
bool __thiscall FUN_004ef6d0(int *param_1,int param_2,undefined4 param_3,void *param_4)

{
  int iVar1;
  bool bVar2;
  
  bVar2 = -1 < param_2;
  if ((bVar2) && (param_2 != 0)) {
    iVar1 = FUN_004ee2d0(param_1,(short)param_1[0x25] + param_2,param_4);
    bVar2 = iVar1 != 0;
    iVar1 = (**(code **)(*param_1 + 0x2e0))();
    if (iVar1 != 0) {
      iVar1 = (**(code **)(*param_1 + 0xac))(param_3,param_4);
      if ((iVar1 != 0) && (bVar2)) {
        return true;
      }
      bVar2 = false;
    }
  }
  return bVar2;
}

