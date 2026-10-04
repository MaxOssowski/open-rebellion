
void __thiscall FUN_004ac7a0(int *param_1,int *param_2)

{
  int iVar1;
  int iVar2;
  int iVar3;
  int iVar4;
  void *pvVar5;
  int *piVar6;
  uint *puVar7;
  char *pcVar8;
  uint uVar9;
  uint uStack_38;
  void *pvStack_34;
  int *piStack_30;
  undefined4 uStack_2c;
  int iStack_28;
  undefined1 auStack_1c [16];
  void *pvStack_c;
  undefined1 *puStack_8;
  int iStack_4;
  
  iStack_4 = 0xffffffff;
  puStack_8 = &LAB_006385fb;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  iVar4 = FUN_00422ca0();
  piVar6 = param_2;
  if (iVar4 != 0) {
    piVar6 = *(int **)(iVar4 + 0x9c);
  }
  iVar4 = (**(code **)(*param_2 + 0x28))();
  puVar7 = &uStack_38;
  pvVar5 = (void *)thunk_FUN_005f5060(iVar4);
  FUN_00403040(pvVar5,puVar7);
  iStack_4 = 0;
  piStack_30 = FUN_004f2d10((int)piVar6,&uStack_38);
  if (piStack_30 != (int *)0x0) {
    param_1[0x4f] = (int)param_2;
    (**(code **)(*param_1 + 0x80))();
    piVar6 = (int *)(**(code **)(*param_1 + 0x88))(auStack_1c);
    iVar4 = *piVar6;
    iStack_28 = piVar6[1];
    iVar1 = piVar6[2];
    iVar2 = piVar6[3];
    pvStack_34 = (void *)FUN_00618b70(0xf4);
    iStack_4._0_1_ = 1;
    if (pvStack_34 == (void *)0x0) {
      piVar6 = (int *)0x0;
    }
    else {
      iVar3 = param_1[7];
      uVar9 = 0;
      puVar7 = (uint *)FUN_006073d0(param_1);
      piVar6 = FUN_00604cf0(pvStack_34,iVar3,iVar4,iStack_28,iVar1 - iVar4,iVar2 - iStack_28,
                            (int)param_1,puVar7,uVar9);
    }
    iStack_4._0_1_ = 0;
    param_1[0x4e] = (int)piVar6;
    if (piVar6 != (int *)0x0) {
      (**(code **)(*piVar6 + 0x18))(10);
      pvVar5 = FUN_004f6270(piStack_30,&uStack_2c);
      iStack_4._0_1_ = 2;
      pcVar8 = (char *)FUN_00583c40((int)pvVar5);
      FUN_00604f90((void *)param_1[0x4e],pcVar8);
      iStack_4 = (uint)iStack_4._1_3_ << 8;
      FUN_005f2ff0(&uStack_2c);
      FUN_005ffce0((void *)param_1[0x4e],0);
      FUN_00605110((void *)param_1[0x4e],0,-1);
      SetWindowPos(*(HWND *)(param_1[0x4e] + 0x18),(HWND)0x0,0,0,0,0,3);
      SetFocus(*(HWND *)(param_1[0x4e] + 0x18));
    }
  }
  iStack_4 = 0xffffffff;
  FUN_00619730();
  ExceptionList = pvStack_c;
  return;
}

