
bool __thiscall FUN_00576700(void *param_1,void *param_2)

{
  int iVar1;
  bool bVar2;
  
  bVar2 = true;
  if (*(int *)((int)param_1 + 0x60) == 0) {
    iVar1 = FUN_00521880(param_1,2,param_2);
    bVar2 = iVar1 != 0;
  }
  return bVar2;
}

