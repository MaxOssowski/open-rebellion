
bool __thiscall FUN_00589490(int param_1,void *param_2,uint *param_3,void *param_4)

{
  uint *puVar1;
  bool bVar2;
  undefined3 extraout_var;
  
  puVar1 = param_3;
  *param_3 = 0;
  param_3 = (uint *)0x0;
  bVar2 = FUN_00588e80(*(void **)(param_1 + 4),param_2,(uint *)&param_3,param_4);
  *puVar1 = (uint)param_3;
  return CONCAT31(extraout_var,bVar2) != 0;
}

