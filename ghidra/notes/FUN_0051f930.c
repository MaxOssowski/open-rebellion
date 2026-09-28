// FUN_0051f930

undefined4 __cdecl FUN_0051f930(int param_1,undefined4 param_2)

{
  int *piVar1;
  int iVar2;
  bool bVar3;
  
  bVar3 = true;
  iVar2 = 0;
  piVar1 = &DAT_006b6fe0;
  do {
    if (DAT_006b8fd8 <= iVar2) break;
    bVar3 = *piVar1 != param_1;
    iVar2 = iVar2 + 1;
    piVar1 = piVar1 + 2;
  } while (bVar3);
  if ((bVar3) && (DAT_006b8fd8 + 1 < 0x3ff)) {
    (&DAT_006b6fe0)[DAT_006b8fd8 * 2] = param_1;
    (&DAT_006b6fe4)[DAT_006b8fd8 * 2] = param_2;
    DAT_006b8fd8 = DAT_006b8fd8 + 1;
  }
  return 0;
}

