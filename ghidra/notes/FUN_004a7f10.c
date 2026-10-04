
void __thiscall FUN_004a7f10(void *this,uint *param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  
  puVar1 = FUN_004f5940((void *)((int)this + 0x114),param_1);
  if (puVar1 != (undefined4 *)0x0) {
    FUN_004acca0((void *)((int)this + 0x114),puVar1);
    (**(code **)(*(int *)this + 100))(param_1,param_2);
  }
  return;
}

