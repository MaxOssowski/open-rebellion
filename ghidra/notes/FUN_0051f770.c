
undefined4 __cdecl FUN_0051f770(int param_1,undefined4 param_2)

{
  int *piVar1;
  int iVar2;
  bool bVar3;
  
  bVar3 = true;
  iVar2 = 0;
  piVar1 = &DAT_006b2fd8;
  do {
    if (DAT_006b6fd8 <= iVar2) break;
    bVar3 = *piVar1 != param_1;
    iVar2 = iVar2 + 1;
    piVar1 = piVar1 + 2;
  } while (bVar3);
  if ((bVar3) && (DAT_006b6fd8 + 1U < 0x800)) {
    (&DAT_006b2fd8)[DAT_006b6fd8 * 2] = param_1;
    (&DAT_006b2fdc)[DAT_006b6fd8 * 2] = param_2;
    DAT_006b6fd8 = DAT_006b6fd8 + 1;
  }
  return 0;
}

