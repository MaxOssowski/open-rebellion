
undefined4 __fastcall FUN_005011f0(int *param_1)

{
  bool bVar1;
  undefined3 extraout_var;
  int iVar2;
  int iVar3;
  undefined4 uVar4;
  uint uVar5;
  undefined3 extraout_var_00;
  
  uVar4 = 0;
  bVar1 = FUN_00500680(param_1);
  if (CONCAT31(extraout_var,bVar1) != 0) {
    uVar5 = (uint)param_1[0x19] >> 0x10 & 0xf;
    bVar1 = FUN_00500680(param_1);
    iVar2 = CONCAT31(extraout_var_00,bVar1);
    iVar3 = (**(code **)(*param_1 + 0x1ec))();
    uVar4 = FUN_0053e170(iVar3,uVar5,iVar2);
  }
  return uVar4;
}

