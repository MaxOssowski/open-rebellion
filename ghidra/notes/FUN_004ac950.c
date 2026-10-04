
void __fastcall FUN_004ac950(int param_1)

{
  uint uVar1;
  char *pcVar2;
  undefined4 auStack_18 [3];
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_00638618;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  SetFocus(*(HWND *)(param_1 + 0x18));
  if ((*(int *)(param_1 + 0x138) != 0) && (*(int *)(param_1 + 0x13c) != 0)) {
    uVar1 = FUN_005f3070(*(int *)(param_1 + 0x138) + 0x98);
    if ((short)uVar1 != 0) {
      pcVar2 = (char *)FUN_00583c40(*(int *)(param_1 + 0x138) + 0x98);
      FUN_005f35b0(auStack_18,pcVar2);
      uStack_4 = 0;
      FUN_005f3090((void *)(*(int *)(param_1 + 0x13c) + 0x44),(int)auStack_18);
      uStack_4 = 0xffffffff;
      FUN_005f2ff0(auStack_18);
      FUN_0041ce20(*(int **)(param_1 + 0x13c),0);
      *(undefined4 *)(param_1 + 0x13c) = 0;
      FUN_00600280(*(int *)(param_1 + 0x138));
      if (*(undefined4 **)(param_1 + 0x138) != (undefined4 *)0x0) {
        (**(code **)**(undefined4 **)(param_1 + 0x138))(1);
      }
      *(undefined4 *)(param_1 + 0x138) = 0;
    }
  }
  ExceptionList = pvStack_c;
  return;
}

