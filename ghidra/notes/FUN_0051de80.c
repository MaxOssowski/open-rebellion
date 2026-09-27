
uint __fastcall FUN_0051de80(int param_1)

{
  uint uVar1;
  
  uVar1 = 0;
  if (*(int *)(param_1 + 0xc0) != 0) {
    uVar1 = *(uint *)(*(int *)(param_1 + 0xc0) + 0x74) & 0x10000000;
  }
  return uVar1;
}

