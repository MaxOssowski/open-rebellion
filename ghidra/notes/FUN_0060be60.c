
void __thiscall FUN_0060be60(void *this,int param_1,int param_2)

{
  undefined4 *puVar1;
  
  if (param_1 != 0) {
    puVar1 = *(undefined4 **)((int)this + 0x20);
    if (puVar1 != (undefined4 *)0x0) {
      FUN_005fbfa0(puVar1);
      FUN_00618b60((undefined *)puVar1);
    }
    *(int *)((int)this + 0x20) = param_1;
  }
  if (param_2 != 0) {
    puVar1 = *(undefined4 **)((int)this + 0x24);
    if (puVar1 != (undefined4 *)0x0) {
      FUN_005fbfa0(puVar1);
      FUN_00618b60((undefined *)puVar1);
    }
    *(int *)((int)this + 0x24) = param_2;
  }
  return;
}

