// FUN_0053f950

uint __cdecl
FUN_0053f950(int param_1,int *param_2,undefined4 param_3,undefined4 param_4,void *param_5)

{
  uint uVar1;
  bool bVar2;
  
  if ((param_1 < 0x300) || (799 < param_1)) {
    bVar2 = false;
  }
  else {
    bVar2 = true;
  }
  uVar1 = 0;
  if (bVar2) {
    uVar1 = FUN_0053ff90(param_1,param_2,param_5,&param_1);
    if ((uVar1 != 0) && (param_1 != 0)) {
      *(undefined4 *)(param_1 + 0x4c) = param_3;
      *(undefined4 *)(param_1 + 0x50) = param_4;
      FUN_0053fcb0(param_1);
    }
  }
  return uVar1;
}

