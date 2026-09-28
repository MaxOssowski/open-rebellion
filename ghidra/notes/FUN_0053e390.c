
int __cdecl FUN_0053e390(int param_1,undefined4 *param_2)

{
  void *this;
  undefined4 uVar1;
  int iVar2;
  
  *param_2 = 0;
  this = (void *)FUN_0055bed0(0x3c,param_1);
  if ((this == (void *)0x0) || (*(int *)((int)this + 0x20) != param_1)) {
    iVar2 = 0;
  }
  else {
    iVar2 = 1;
  }
  if (iVar2 != 0) {
    uVar1 = FUN_00585840(this,DAT_006b904c);
    *param_2 = uVar1;
  }
  return iVar2;
}

