
void __thiscall FUN_0044f5e0(int *param_1,short param_2)

{
  if (param_2 == 0x14) {
    FUN_0041ce20(*(int **)(param_1[0x46] + 0x5c),1);
  }
  else {
    if (param_2 != 0x15) {
      return;
    }
    if (*(undefined4 **)(param_1[0x46] + 0x5c) != (undefined4 *)0x0) {
      (**(code **)**(undefined4 **)(param_1[0x46] + 0x5c))(1);
      (**(code **)(*param_1 + 0x30))();
      return;
    }
  }
  (**(code **)(*param_1 + 0x30))();
  return;
}

