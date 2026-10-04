
void __thiscall FUN_004a7390(int *param_1,int param_2,undefined4 param_3)

{
  if (param_2 != 0x1b) {
    (**(code **)(*(int *)param_1[8] + 0x14))((int *)param_1[8],0x100,param_2,param_3);
    return;
  }
  (**(code **)(*param_1 + 0x30))();
  return;
}

