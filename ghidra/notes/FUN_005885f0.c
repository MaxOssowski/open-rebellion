// FUN_005885f0

undefined4 __thiscall FUN_005885f0(void *this,undefined4 param_1)

{
  bool bVar1;
  bool bVar2;
  undefined3 extraout_var;
  undefined3 extraout_var_00;
  undefined3 extraout_var_01;
  
  bVar1 = FUN_00588470(this,(undefined4 *)((int)this + 0x34),param_1);
  bVar2 = FUN_005884f0(this,(undefined4 *)((int)this + 0x2c),param_1);
  if ((CONCAT31(extraout_var_00,bVar2) == 0) || (CONCAT31(extraout_var,bVar1) == 0)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  bVar2 = FUN_00588570(this,(undefined4 *)((int)this + 0x30),param_1);
  if ((CONCAT31(extraout_var_01,bVar2) != 0) && (bVar1)) {
    return 1;
  }
  return 0;
}

