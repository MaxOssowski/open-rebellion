
undefined4 FUN_0055bf10(void)

{
  bool bVar1;
  int iVar2;
  int iVar3;
  
  iVar2 = FUN_0053e390(0x1c00,&DAT_006bb50c);
  iVar3 = FUN_0053e390(0x1c01,&DAT_006bb510);
  if ((iVar3 == 0) || (iVar2 == 0)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  iVar2 = FUN_0053e390(0x1c02,&DAT_006bb508);
  if ((iVar2 != 0) && (bVar1)) {
    return 1;
  }
  return 0;
}

