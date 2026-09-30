
void __thiscall
FUN_004b7150(int param_1,int *param_2,undefined4 *param_3,int *param_4,undefined4 *param_5)

{
  int iVar1;
  void *pvVar2;
  
  *param_2 = 0;
  *param_3 = 0;
  *param_4 = 0;
  *param_5 = 0;
  pvVar2 = FUN_004f5940((void *)(*(int *)(param_1 + 0xc) + 0x44),
                        (uint *)(*(int *)(param_1 + 0x10) + 0x34));
  if (pvVar2 != (void *)0x0) {
    if (((*param_2 == 0) && (*(int *)(*(int *)(param_1 + 0x10) + 0xa8) < 2)) &&
       ((*(byte *)((int)pvVar2 + 0x2c) & 0x18) != 0)) {
      *param_2 = 0x54;
    }
    else if ((*param_4 == 0) &&
            ((iVar1 = *(int *)(*(int *)(param_1 + 0x10) + 0xa8), 2 < iVar1 ||
             (((*(byte *)((int)pvVar2 + 0x2c) & 0x18) == 0 && (0 < iVar1)))))) {
      *param_4 = 0x54;
    }
    if ((*param_2 == 0) && (*(int *)(*(int *)(param_1 + 0x10) + 0xa4) == 0)) {
      *param_2 = 0x52;
    }
    else if ((*param_4 == 0) && (1 < *(int *)(*(int *)(param_1 + 0x10) + 0xa4))) {
      *param_4 = 0x52;
    }
    if (((*param_2 == 0) && (*(int *)(*(int *)(param_1 + 0x10) + 0xac) == 0)) &&
       ((*(byte *)((int)pvVar2 + 0x2c) & 8) != 0)) {
      *param_2 = 0x69;
    }
    else if ((*param_4 == 0) &&
            ((iVar1 = *(int *)(*(int *)(param_1 + 0x10) + 0xac), 1 < iVar1 ||
             (((*(byte *)((int)pvVar2 + 0x2c) & 8) == 0 && (0 < iVar1)))))) {
      *param_4 = 0x69;
    }
    if (((*param_2 == 0) && (*(int *)(*(int *)(param_1 + 0x10) + 0xa0) == 0)) &&
       ((*(uint *)((int)pvVar2 + 0x2c) & 0x40008) != 0)) {
      *param_3 = 4;
      *param_2 = (*(int *)(param_1 + 4) != 1) + 0x62;
      return;
    }
    if ((*param_4 == 0) &&
       ((iVar1 = *(int *)(*(int *)(param_1 + 0x10) + 0xa0), 1 < iVar1 ||
        (((*(uint *)((int)pvVar2 + 0x2c) & 0x40008) == 0 && (0 < iVar1)))))) {
      *param_5 = 4;
      *param_4 = (*(int *)(param_1 + 4) != 1) + 0x62;
    }
  }
  return;
}

