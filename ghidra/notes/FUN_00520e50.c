
uint __fastcall FUN_00520e50(int param_1)

{
  int iVar1;
  
  iVar1 = *(int *)(param_1 + 0x54);
  if (*(int *)(iVar1 + 0x18) != 0) {
    switch(*(undefined4 *)(param_1 + 0x68)) {
    case 2:
      return *(uint *)(iVar1 + 0x34) & 0xf;
    case 3:
      return *(uint *)(iVar1 + 0x34) >> 4 & 0xf;
    default:
      break;
    case 7:
      return *(uint *)(iVar1 + 0x34) >> 8 & 0xf;
    case 9:
      return *(uint *)(iVar1 + 0x34) >> 0xc & 0xf;
    }
  }
  return 0;
}

