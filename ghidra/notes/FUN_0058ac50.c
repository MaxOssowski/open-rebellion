// FUN_0058ac50

undefined4 FUN_0058ac50(uint *param_1,void *param_2,undefined4 *param_3)

{
  void *pvVar1;
  
  FUN_004ece80(param_3);
  pvVar1 = FUN_004f5940(param_2,param_1);
  if ((pvVar1 != (void *)0x0) && (*(int *)((int)pvVar1 + 0x1c) == 0)) {
    *(undefined4 *)((int)pvVar1 + 0x1c) = DAT_0066a7e4;
    FUN_004f26d0(param_3,param_1);
  }
  return 1;
}

