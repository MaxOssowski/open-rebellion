
undefined4 __thiscall
FUN_005896e0(int param_1,int *param_2,void *param_3,int *param_4,undefined4 param_5)

{
  int aiStack_18 [2];
  undefined4 uStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_0064faa8;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  *param_4 = 0;
  if (*(int *)((int)*(void **)(param_1 + 4) + 0x2c) != 0) {
    if (*(int *)(param_1 + 0xc) == 0) {
      *(undefined4 *)(param_1 + 0xc) = 1;
      FUN_005887a0(*(void **)(param_1 + 4),(int *)(param_1 + 0x10),param_5);
      FUN_005872a0(aiStack_18,*(undefined4 *)(param_1 + 4));
      uStack_4 = 0;
      FUN_00587b70(*(void **)(param_1 + 4),aiStack_18,param_5);
      *(undefined4 *)(param_1 + 0x14) = uStack_10;
      uStack_4 = 0xffffffff;
      FUN_005872e0(aiStack_18);
    }
    if (*param_4 == 0) {
      FUN_00588a90(*(void **)(param_1 + 4),param_2,param_3,*(int *)(param_1 + 0x10),
                   *(int *)(param_1 + 0x14),(undefined4 *)(param_1 + 8));
    }
  }
  *param_4 = *(int *)(param_1 + 8);
  ExceptionList = pvStack_c;
  return 1;
}

