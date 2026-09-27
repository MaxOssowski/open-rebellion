
bool __thiscall FUN_0051eaa0(void *this,int *param_1)

{
  bool bVar1;
  
  bVar1 = false;
  if ((param_1 != (int *)0x0) && (bVar1 = *(int *)((int)this + 0xb4) != 0, bVar1)) {
    (**(code **)(*param_1 + 0x14))();
    FUN_00583c50(*(void **)((int)this + 0xb4),(int)param_1);
    FUN_0051ebb0();
  }
  return bVar1;
}

