
bool __thiscall FUN_0056cf30(void *param_1,void *param_2)

{
  uint uVar1;
  int iVar2;
  void *this;
  bool bVar3;
  int iStack_10;
  int iStack_c;
  int iStack_8;
  int iStack_4;
  
  uVar1 = *(uint *)((int)param_1 + 0x24) >> 6 & 3;
  if (uVar1 == 1) {
    iVar2 = 1;
  }
  else if (uVar1 == 2) {
    iVar2 = 2;
  }
  else {
    iVar2 = 0;
  }
  this = (void *)0x0;
  bVar3 = false;
  if (iVar2 != 0) {
    this = (void *)FUN_00506f30(iVar2);
    bVar3 = this != (void *)0x0;
  }
  if (this != (void *)0x0) {
    iStack_10 = 0;
    iVar2 = FUN_0056c0e0(param_1,&iStack_10);
    if ((iVar2 == 0) || (bVar3 == false)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    iStack_4 = 0;
    iVar2 = FUN_0052eac0(this,iStack_10,&iStack_4);
    if ((iVar2 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    iStack_c = 0;
    iVar2 = FUN_0052ea70(this,iStack_10,&iStack_c);
    if ((iVar2 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    iVar2 = FUN_00530600(this,iStack_10,iStack_c + *(int *)((int)param_1 + 0xa8),param_2);
    if ((iVar2 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    iStack_8 = 0;
    iVar2 = FUN_0052eac0(this,iStack_10,&iStack_8);
    if ((iVar2 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    iVar2 = FUN_0056c2b0(param_1,iStack_8 - iStack_4,param_2);
    if ((iVar2 == 0) || (!bVar3)) {
      bVar3 = false;
    }
    else {
      bVar3 = true;
    }
    iVar2 = FUN_00521880(param_1,(*(int *)((int)param_1 + 0xa8) != 0) + 2,param_2);
    if ((iVar2 != 0) && (bVar3)) {
      return true;
    }
    bVar3 = false;
  }
  return bVar3;
}

