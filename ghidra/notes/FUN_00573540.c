
int __thiscall
FUN_00573540(void *param_1,int *param_2,int param_3,undefined4 *param_4,void *param_5)

{
  undefined4 *puVar1;
  void *pvVar2;
  bool bVar3;
  undefined3 extraout_var;
  undefined3 extraout_var_00;
  int iVar4;
  int iVar5;
  
  puVar1 = param_4;
  iVar5 = 1;
  *param_4 = 0;
  bVar3 = FUN_0053e2f0(param_3);
  pvVar2 = param_5;
  if (CONCAT31(extraout_var,bVar3) != 0) {
    *puVar1 = 1;
    iVar5 = FUN_00521880(param_1,3,param_5);
    param_4 = (undefined4 *)0x0;
    bVar3 = FUN_00521070(param_1,(int *)&param_4);
    if ((CONCAT31(extraout_var_00,bVar3) == 0) || (iVar5 == 0)) {
      iVar5 = 0;
    }
    else {
      iVar5 = 1;
    }
    if ((param_4 != (undefined4 *)0x0) &&
       (((*(uint *)((int)param_1 + 0x24) ^ *(uint *)((int)param_4 + 0x24)) & 0xc0) != 0)) {
      iVar4 = (**(code **)(*param_2 + 0x1d8))();
      if (iVar4 != 0) {
        iVar4 = FUN_00533ea0(param_2,*(short *)((int)param_2 + 0x5a) + DAT_006bb534,pvVar2);
        if ((iVar4 != 0) && (iVar5 != 0)) {
          return 1;
        }
        iVar5 = 0;
      }
    }
  }
  return iVar5;
}

