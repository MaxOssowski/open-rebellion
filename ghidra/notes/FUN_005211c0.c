// FUN_005211c0

bool __thiscall FUN_005211c0(void *this,undefined4 *param_1)

{
  uint uVar1;
  int *piVar2;
  int iVar3;
  bool bVar4;
  
  *param_1 = 0;
  uVar1 = *(uint *)((int)this + 0x24) >> 6 & 3;
  if (uVar1 == 1) {
    iVar3 = 1;
  }
  else if (uVar1 == 2) {
    iVar3 = 2;
  }
  else {
    iVar3 = 0;
  }
  bVar4 = false;
  if (iVar3 != 0) {
    piVar2 = FUN_004f2d10(iVar3,(uint *)((int)this + 0x74));
    bVar4 = piVar2 != (int *)0x0;
    *param_1 = piVar2;
  }
  return bVar4;
}

