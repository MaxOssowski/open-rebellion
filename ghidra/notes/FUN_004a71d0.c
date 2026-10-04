
void __thiscall FUN_004a71d0(int param_1,int *param_2)

{
  int *piVar1;
  void *this;
  int iStack_20;
  int iStack_1c;
  int iStack_18;
  int iStack_14;
  int aiStack_10 [4];
  
  FUN_004acb20(&iStack_20);
  if ((*(byte *)(param_1 + 0x14c) & 1) == 0) {
    piVar1 = *(int **)(param_1 + 0x188);
    this = *(void **)(param_1 + 0x160);
  }
  else {
    piVar1 = *(int **)(param_1 + 0x18c);
    this = *(void **)(param_1 + 0x164);
  }
  piVar1 = (int *)(**(code **)(*piVar1 + 8))();
  if (piVar1 != (int *)0x0) {
    do {
      if ((*(byte *)(piVar1 + 0xf) & 1) != 0) break;
      piVar1 = (int *)(**(code **)(*piVar1 + 0xc))();
    } while (piVar1 != (int *)0x0);
    if (piVar1 != (int *)0x0) {
      piVar1 = (int *)FUN_0060a2a0(this,aiStack_10,(int)piVar1);
      iStack_20 = *piVar1 + *(int *)((int)this + 0x28);
      iStack_1c = piVar1[1] + *(int *)((int)this + 0x2c);
      iStack_14 = piVar1[3] + *(int *)((int)this + 0x2c);
      iStack_18 = *(int *)((int)this + 0x30) + -8 + *(int *)((int)this + 0x28);
    }
  }
  *param_2 = iStack_20;
  param_2[1] = iStack_1c;
  param_2[2] = iStack_18;
  param_2[3] = iStack_14;
  return;
}

