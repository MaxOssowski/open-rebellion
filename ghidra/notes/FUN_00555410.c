
uint FUN_00555410(uint *param_1,uint *param_2,uint *param_3)

{
  int *piVar1;
  int *piVar2;
  uint uVar3;
  
  piVar1 = (int *)FUN_00504dc0(param_1);
  if (piVar1 == (int *)0x0) {
    uVar3 = 0;
  }
  else {
    piVar2 = (int *)FUN_00504dc0(param_2);
    uVar3 = FUN_00555460(piVar1,piVar2,param_3);
  }
  return uVar3;
}

