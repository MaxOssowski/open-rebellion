
void __thiscall FUN_00604500(void *this,uint param_1)

{
  int iVar1;
  bool bVar2;
  
  iVar1 = *(int *)((int)this + 4);
  bVar2 = true;
  while ((iVar1 != 0 && (bVar2))) {
    if (param_1 < *(uint *)(iVar1 + 0x24)) {
      iVar1 = *(int *)(iVar1 + 4);
    }
    else if (*(uint *)(iVar1 + 0x24) < param_1) {
      iVar1 = *(int *)(iVar1 + 8);
    }
    else {
      bVar2 = false;
    }
  }
  return;
}

