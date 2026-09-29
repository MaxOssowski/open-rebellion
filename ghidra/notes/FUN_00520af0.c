// FUN_00520af0

undefined4 __fastcall FUN_00520af0(int param_1)

{
  bool bVar1;
  int iVar2;
  undefined3 extraout_var;
  
  iVar2 = FUN_00520ac0(param_1);
  if (iVar2 < 5) {
    bVar1 = FUN_00520bb0(param_1);
    if (CONCAT31(extraout_var,bVar1) != 0) {
      return 0;
    }
  }
  return 1;
}

