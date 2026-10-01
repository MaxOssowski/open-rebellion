
int * __thiscall FUN_0045c660(void *this,LONG param_1,LONG param_2)

{
  POINT pt;
  int *piVar1;
  BOOL BVar2;
  
  piVar1 = (int *)(**(code **)(*(int *)((int)this + 0x164) + 8))();
  while ((piVar1 != (int *)0x0 &&
         (pt.y = param_2, pt.x = param_1, BVar2 = PtInRect((RECT *)(piVar1 + 0x10),pt), BVar2 == 0))
        ) {
    piVar1 = (int *)(**(code **)(*piVar1 + 0xc))();
  }
  return piVar1;
}

