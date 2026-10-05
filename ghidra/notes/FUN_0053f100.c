
int * __cdecl FUN_0053f100(uint *param_1)

{
  int iVar1;
  int *piVar2;
  
  piVar2 = (int *)0x0;
  if ((*param_1 & 0xff000000) != 0) {
    iVar1 = FUN_0051cab0(param_1);
    if (iVar1 != 0) {
      piVar2 = (int *)FUN_00540500(*param_1 >> 0x18);
      if (piVar2 != (int *)0x0) {
        (**(code **)(*piVar2 + 0x28))(iVar1);
      }
    }
  }
  return piVar2;
}

