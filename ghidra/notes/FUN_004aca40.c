
void __fastcall FUN_004aca40(int param_1)

{
  SetFocus(*(HWND *)(param_1 + 0x18));
  if (*(int *)(param_1 + 0x138) != 0) {
    DestroyWindow(*(HWND *)(*(int *)(param_1 + 0x138) + 0x18));
    if (*(undefined4 **)(param_1 + 0x138) != (undefined4 *)0x0) {
      (**(code **)**(undefined4 **)(param_1 + 0x138))(1);
    }
    *(undefined4 *)(param_1 + 0x138) = 0;
  }
  return;
}

