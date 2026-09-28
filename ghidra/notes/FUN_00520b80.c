// FUN_00520b80

undefined4 __fastcall FUN_00520b80(int param_1)

{
  if (*(int *)(param_1 + 0x2c) != 0) {
    return *(undefined4 *)(*(int *)(param_1 + 0x2c) + 0x60);
  }
  return 0;
}

