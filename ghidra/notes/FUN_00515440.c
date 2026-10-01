
bool __cdecl FUN_00515440(uint *param_1,uint *param_2,void *param_3)

{
  int iVar1;
  bool bVar2;
  
  iVar1 = thunk_FUN_00506e60();
  bVar2 = iVar1 != 0;
  if (iVar1 != 0) {
    iVar1 = FUN_00556390(param_1,param_2,param_3);
    if ((iVar1 != 0) && (bVar2)) {
      return true;
    }
    bVar2 = false;
  }
  return bVar2;
}

