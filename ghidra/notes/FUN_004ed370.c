
int __fastcall FUN_004ed370(int param_1)

{
  int iVar1;
  
  iVar1 = 0;
  if ((*(byte *)(param_1 + 0x50) & 0x40) != 0) {
    iVar1 = (int)*(short *)(param_1 + 0x9a);
  }
  return iVar1;
}

