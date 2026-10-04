
void __thiscall FUN_004aa2b0(int *param_1,int param_2,undefined4 param_3)

{
  int iVar1;
  BOOL BVar2;
  uint uVar3;
  
  if (param_2 == 0x1b) {
    (**(code **)(*param_1 + 0x30))();
  }
  else {
    if ((param_2 != 0x25) && (param_2 != 0x27)) {
      (**(code **)(*(int *)param_1[8] + 0x14))((int *)param_1[8],0x100,param_2,param_3);
      return;
    }
    iVar1 = *(int *)(param_1[0x58] + 0x94);
    if (iVar1 == 0) {
      uVar3 = 0;
    }
    else {
      uVar3 = *(uint *)(iVar1 + 0x24);
    }
    iVar1 = FUN_00604500((void *)(param_1[0x58] + 0x6c),uVar3);
    if (iVar1 != 0) {
      if (param_2 == 0x25) {
        iVar1 = FUN_005f5c60(iVar1);
      }
      else {
        iVar1 = *(int *)(iVar1 + 0x10);
      }
      if (iVar1 != 0) {
        do {
          BVar2 = IsWindowVisible(*(HWND *)(iVar1 + 0x18));
          if (BVar2 == 0) {
            if (param_2 == 0x25) {
              iVar1 = FUN_005f5c60(iVar1);
            }
            else {
              iVar1 = *(int *)(iVar1 + 0x10);
            }
          }
          else {
            FUN_0060d7e0((void *)param_1[0x58],*(uint *)(iVar1 + 0x24),1);
            iVar1 = 0;
          }
        } while (iVar1 != 0);
        return;
      }
    }
  }
  return;
}

