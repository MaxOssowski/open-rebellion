
void __thiscall FUN_004a4ac0(void *this,int *param_1)

{
  int iVar1;
  int iVar2;
  
  iVar1 = *param_1;
  iVar2 = *(int *)(iVar1 + 8);
  while (iVar1 != 0) {
    FUN_005f5ac0(*(void **)((int)this + 0x188),iVar1);
    if ((undefined4 *)*param_1 != (undefined4 *)0x0) {
      (*(code *)**(undefined4 **)*param_1)(1);
    }
    *param_1 = 0;
    if ((iVar2 != 0) && ((*(byte *)(iVar2 + 0x68) & 4) == 0)) {
      *param_1 = iVar2;
      iVar2 = *(int *)(iVar2 + 8);
    }
    iVar1 = *param_1;
  }
  *param_1 = iVar2;
  return;
}

