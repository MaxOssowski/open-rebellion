
int __cdecl FUN_0053fdd0(uint param_1,void *param_2,int param_3,int param_4)

{
  bool bVar1;
  undefined3 extraout_var;
  int iVar2;
  
  if ((param_1 < 0x100) || (0x242 < param_1)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  iVar2 = 0;
  if (bVar1) {
    bVar1 = FUN_005400f0(param_1,param_2,param_4,(int *)&param_1);
    iVar2 = CONCAT31(extraout_var,bVar1);
    if ((iVar2 != 0) && (param_1 != 0)) {
      *(uint *)(param_1 + 0x24) = (uint)(param_3 == 0);
      *(int *)(param_1 + 0x28) = param_3;
      FUN_004fd3b0(param_1);
    }
  }
  return iVar2;
}

