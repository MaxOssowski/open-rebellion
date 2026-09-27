
undefined4 __cdecl FUN_0054f100(int param_1)

{
  undefined4 uVar1;
  int *piVar2;
  code *pcVar3;
  int iVar4;
  
  uVar1 = 0;
  pcVar3 = (code *)0x0;
  iVar4 = 0;
  piVar2 = &DAT_006b92fc;
  do {
    if (DAT_006b92f0 <= iVar4) break;
    if (piVar2[-1] == param_1) {
      pcVar3 = (code *)*piVar2;
    }
    iVar4 = iVar4 + 1;
    piVar2 = piVar2 + 2;
  } while (pcVar3 == (code *)0x0);
  if (pcVar3 != (code *)0x0) {
    uVar1 = (*pcVar3)();
  }
  return uVar1;
}

