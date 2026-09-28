// FUN_00589620

undefined4 __thiscall
FUN_00589620(void *this,int *param_1,void *param_2,uint *param_3,void *param_4)

{
  uint *puVar1;
  void *pvVar2;
  
  pvVar2 = param_4;
  puVar1 = param_3;
  *param_3 = 0;
  param_3 = (uint *)0x0;
  FUN_00588700(*(void **)((int)this + 4),&param_3,param_4);
  *puVar1 = (uint)(param_3 == (uint *)0x0);
  if ((param_3 == (uint *)0x0) == 0) {
    FUN_00588b90(*(void **)((int)this + 4),(int *)param_3,param_1,param_2,pvVar2);
  }
  return 1;
}

