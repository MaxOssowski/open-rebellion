
int __fastcall FUN_00586130(int param_1)

{
  int iVar1;
  
  iVar1 = FUN_0053e290(*(uint *)(param_1 + 0x48) >> 1 & 0x7fff);
  return iVar1 + (*(uint *)(param_1 + 0x48) >> 0x10);
}

