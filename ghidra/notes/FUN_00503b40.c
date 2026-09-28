// FUN_00503b40

undefined4 __fastcall FUN_00503b40(int param_1)

{
  undefined4 uVar1;
  
  uVar1 = 0;
  if (*(int *)(param_1 + 0x2c) != 0) {
    uVar1 = *(undefined4 *)(*(int *)(param_1 + 0x2c) + 0x98);
  }
  return uVar1;
}

