
bool __thiscall FUN_00576e50(int *param_1,int *param_2,undefined4 *param_3)

{
  int iVar1;
  int iVar2;
  bool bVar3;
  int *piStack_4;
  
  piStack_4 = param_1;
  iVar1 = FUN_00586c80(param_1,(int *)&piStack_4);
  bVar3 = iVar1 != 0;
  if (piStack_4 != (int *)0x0) {
    iVar1 = (**(code **)(*piStack_4 + 0x1f0))();
    iVar2 = (**(code **)(*param_2 + 0x1f0))();
    iVar1 = FUN_0055c810(iVar2,iVar1,param_3);
    if ((iVar1 != 0) && (bVar3)) {
      return true;
    }
    bVar3 = false;
  }
  return bVar3;
}

