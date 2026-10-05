
UINT FUN_00462a50(int *param_1,uint param_2,HPALETTE param_3,HWND param_4)

{
  int *piVar1;
  short sVar2;
  int iVar3;
  undefined4 *puVar4;
  void *this;
  UINT UVar5;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  piVar1 = param_1;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_006313b8;
  pvStack_c = ExceptionList;
  UVar5 = 0;
  if (param_2 == 7) {
    ExceptionList = &pvStack_c;
    SetFocus(*(HWND *)(param_1[0x54] + 0x18));
  }
  else if (param_2 == 0x407) {
    ExceptionList = &pvStack_c;
    FUN_004ece30(&param_2);
    piVar1 = param_1;
    uStack_4 = 2;
    if ((short)param_3 == 0xcd) {
      if (param_1[0x5b] != param_2) {
        FUN_00429440((void *)param_1[0x56],(uint *)(param_1 + 0x5b));
        (**(code **)(*piVar1 + 0x30))();
      }
    }
    uStack_4 = 0xffffffff;
    FUN_00619730();
  }
  else if (param_2 == 0x408) {
    if ((short)param_3 == 0xcd) {
      this = (void *)0x0;
      iVar3 = param_1[0x54];
      ExceptionList = &pvStack_c;
      sVar2 = FUN_005f3040(iVar3 + 0x98);
      if (sVar2 != 0) {
        iVar3 = FUN_00609650((void *)piVar1[0x55],iVar3 + 0x98,0,0);
        this = (void *)FUN_0060a860((void *)piVar1[0x5a],iVar3);
      }
      if (this == (void *)0x0) {
        FUN_004ece30(&param_1);
        uStack_4 = 1;
        FUN_004f26d0(piVar1 + 0x5b,&param_1);
        uStack_4 = 0xffffffff;
        FUN_00619730();
      }
      else {
        puVar4 = FUN_0042d170(this,&param_4);
        uStack_4 = 0;
        FUN_004f26d0(piVar1 + 0x5b,puVar4);
        uStack_4 = 0xffffffff;
        FUN_00619730();
      }
    }
  }
  else {
    ExceptionList = &pvStack_c;
    UVar5 = FUN_00606650(param_1,param_2,param_3,param_4);
  }
  ExceptionList = pvStack_c;
  return UVar5;
}

