
int * __thiscall FUN_00487c50(void *this,int param_1,undefined4 param_2)

{
  int *piVar1;
  
  piVar1 = (int *)FUN_004f5cd0(param_1);
  if (piVar1 != (int *)0x0) {
    piVar1[8] = *(int *)((int)this + 0x24);
    (**(code **)(*piVar1 + 0x24))(param_2);
    (**(code **)(*piVar1 + 0x2c))(param_2);
  }
  return piVar1;
}

