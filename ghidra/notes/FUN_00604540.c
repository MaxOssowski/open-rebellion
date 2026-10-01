
void __thiscall FUN_00604540(void *this,int param_1)

{
  bool bVar1;
  int iVar2;
  
  iVar2 = FUN_005f5060((int)this);
  bVar1 = true;
  while ((iVar2 != 0 && (bVar1))) {
    if (*(int *)(iVar2 + 0x18) == param_1) {
      bVar1 = false;
    }
    else {
      iVar2 = *(int *)(iVar2 + 0x10);
    }
  }
  return;
}

