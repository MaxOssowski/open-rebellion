
void __fastcall FUN_004c21d0(int param_1)

{
  if (*(int *)(param_1 + 0x17c) == 0) {
    *(undefined4 *)(param_1 + 0x17c) = 1;
    *(int *)(*(int *)(param_1 + 0x168) + 0x40) = DAT_006b28cc + 10;
    return;
  }
  *(undefined4 *)(param_1 + 0x17c) = 0;
  *(undefined4 *)(param_1 + 0x174) = 0;
  *(undefined4 *)(param_1 + 0x178) = 0;
  *(int *)(*(int *)(param_1 + 0x168) + 0x44) = DAT_006b28cc + 10;
  return;
}

