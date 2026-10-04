
undefined * __fastcall FUN_004f62d0(int param_1)

{
  undefined *puVar1;
  
  if ((DAT_006b2968 & 1) == 0) {
    DAT_006b2968 = DAT_006b2968 | 1;
    FUN_005f35b0(&DAT_006b2ac8,&DAT_006b120c);
    FUN_00618c20(0x4f6320);
  }
  puVar1 = *(undefined **)(param_1 + 0x34);
  if ((puVar1 == (undefined *)0x0) &&
     (puVar1 = (undefined *)(*(int *)(param_1 + 0x2c) + 0x34), *(int *)(param_1 + 0x2c) == 0)) {
    puVar1 = &DAT_006b2ac8;
  }
  return puVar1;
}

