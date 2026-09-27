
undefined4 __cdecl FUN_0054f140(int param_1,undefined4 param_2)

{
  int *piVar1;
  int iVar2;
  bool bVar3;
  
  bVar3 = true;
  iVar2 = 0;
  piVar1 = &DAT_006b92f8;
  do {
    if (DAT_006b92f0 <= iVar2) break;
    bVar3 = *piVar1 != param_1;
    iVar2 = iVar2 + 1;
    piVar1 = piVar1 + 2;
  } while (bVar3);
  if ((bVar3) && (DAT_006b92f0 + 1 < 0x3ff)) {
    (&DAT_006b92f8)[DAT_006b92f0 * 2] = param_1;
    (&DAT_006b92fc)[DAT_006b92f0 * 2] = param_2;
    DAT_006b92f0 = DAT_006b92f0 + 1;
  }
  return 0;
}

