
void __thiscall FUN_004c2940(void *param_1,int *param_2,uint *param_3)

{
  uint uVar1;
  bool bVar2;
  
  bVar2 = false;
  if (*(int *)((int)param_1 + 0x10) != 0) {
    if ((*param_3 == *(uint *)((int)param_1 + 0x174)) &&
       (param_3[1] == *(uint *)((int)param_1 + 0x178))) {
      FUN_004c43d0((int)param_1);
      return;
    }
    FUN_00520580((void *)((int)param_1 + 0x174),param_3);
    *(undefined4 *)((int)param_1 + 0x17c) = 0;
    uVar1 = *param_3;
    if ((0x8f < uVar1) && (uVar1 < 0x98)) {
      bVar2 = true;
    }
    FUN_00619730();
    if (bVar2) {
      FUN_004c3750(param_1,param_2,(int *)param_3);
      return;
    }
    if ((uVar1 < 0xa0) || (0xaf < uVar1)) {
      bVar2 = false;
    }
    else {
      bVar2 = true;
    }
    FUN_00619730();
    if (bVar2) {
      FUN_004c3860(param_1,param_2,(int *)param_3);
      return;
    }
    if ((uVar1 < 8) || (0xf < uVar1)) {
      bVar2 = false;
    }
    else {
      bVar2 = true;
    }
    FUN_00619730();
    if (bVar2) {
      FUN_004c3920((int)param_1);
      return;
    }
    if ((uVar1 < 0x14) || (0x1b < uVar1)) {
      bVar2 = false;
    }
    else {
      bVar2 = true;
    }
    FUN_00619730();
    if (bVar2) {
      FUN_004c3930(param_2);
      return;
    }
    if ((uVar1 < 0xf3) || (0xf3 < uVar1)) {
      bVar2 = false;
    }
    else {
      bVar2 = true;
    }
    FUN_00619730();
    if (bVar2) {
      FUN_004c3b40(param_1,param_2,(int *)param_3);
      return;
    }
    FUN_004c3bb0(param_1,param_2,(int *)param_3);
  }
  return;
}

