
bool __thiscall FUN_005746e0(void *param_1,int *param_2)

{
  void *pvVar1;
  bool bVar2;
  int iVar3;
  undefined3 extraout_var;
  bool bVar4;
  
  pvVar1 = param_2;
  bVar4 = true;
  if (*(int *)((int)param_1 + 0x60) == 0) {
    iVar3 = FUN_00521880(param_1,2,param_2);
    bVar4 = iVar3 != 0;
  }
  if (*(int *)((int)param_1 + 0x60) == 3) {
    param_2 = (int *)0x0;
    bVar2 = FUN_00521030(param_1,(int *)&param_2);
    if ((CONCAT31(extraout_var,bVar2) == 0) || (bVar4 == false)) {
      bVar4 = false;
    }
    else {
      bVar4 = true;
    }
    if (param_2 != (int *)0x0) {
      iVar3 = (**(code **)(*param_2 + 0xac))(6,pvVar1);
      if ((iVar3 != 0) && (bVar4 != false)) {
        return true;
      }
      bVar4 = false;
    }
  }
  return bVar4;
}

