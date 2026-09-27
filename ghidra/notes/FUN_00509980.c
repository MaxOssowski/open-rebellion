
uint __thiscall FUN_00509980(void *this,uint *param_1)

{
  uint *puVar1;
  char cVar2;
  uint uVar3;
  undefined3 extraout_var;
  
  puVar1 = param_1;
  *param_1 = 0;
  uVar3 = FUN_00509890(this,(uint *)&param_1);
  if (uVar3 != 0) {
    cVar2 = FUN_00509930(this);
    *puVar1 = (uint)((uint *)CONCAT31(extraout_var,cVar2) == param_1);
  }
  return uVar3;
}

