
void __thiscall FUN_0060a860(void *this,int param_1)

{
  int *piVar1;
  
  for (piVar1 = (int *)(**(code **)(*(int *)this + 8))();
      (piVar1 != (int *)0x0 && (piVar1[3] != param_1));
      piVar1 = (int *)(**(code **)(*piVar1 + 0xc))()) {
  }
  return;
}

