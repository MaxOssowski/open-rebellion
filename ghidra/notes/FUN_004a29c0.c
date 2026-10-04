
UINT FUN_004a29c0(int *param_1,uint param_2,HPALETTE param_3,HWND param_4)

{
  HWND hWnd;
  int iVar1;
  HDC pHVar2;
  HDC pHVar3;
  UINT UVar4;
  void *pvVar5;
  uint *puVar6;
  tagPAINTSTRUCT tStack_40;
  
  hWnd = (HWND)param_1[6];
  if (param_2 < 8) {
    if (param_2 == 7) {
      if (param_1[0x4e] == 0) {
        SetFocus(*(HWND *)(param_1[0x58] + 0x18));
        return 0;
      }
      SetFocus(*(HWND *)(param_1[0x4e] + 0x18));
      return 0;
    }
    if (param_2 == 1) {
      pHVar2 = GetDC(hWnd);
      pHVar3 = CreateCompatibleDC(pHVar2);
      param_1[0x6b] = (int)pHVar3;
      ReleaseDC(hWnd,pHVar2);
      FUN_004ac3a0(param_1,1,param_3,param_4);
      PostMessageA(*(HWND *)(param_1[8] + 0x18),0x467,(WPARAM)hWnd,0);
      return 0;
    }
LAB_004a2c25:
    UVar4 = FUN_004ac3a0(param_1,param_2,param_3,param_4);
    return UVar4;
  }
  if (0x111 < param_2) {
    if ((param_2 == 0x201) || (param_2 == 0x204)) {
      PostMessageA(*(HWND *)(param_1[8] + 0x18),0x467,(WPARAM)hWnd,0);
    }
    else if (param_2 == 0x467) {
      InvalidateRect(hWnd,(RECT *)(param_1 + 0x4a),0);
      return 0;
    }
    goto LAB_004a2c25;
  }
  if (param_2 == 0x111) {
    UVar4 = FUN_004a6390(param_1,0x111,param_3,param_4);
    return UVar4;
  }
  if (param_2 != 0xf) goto LAB_004a2c25;
  pHVar2 = BeginPaint(hWnd,&tStack_40);
  if (hWnd == *(HWND *)(param_1[0x52] + 0xb8)) {
    iVar1 = param_1[0xe];
    puVar6 = (uint *)param_1[0x29];
    pvVar5 = (void *)param_1[0x5e];
  }
  else {
    iVar1 = param_1[0xe];
    puVar6 = (uint *)param_1[0x29];
    pvVar5 = (void *)param_1[0x5f];
  }
  FUN_005fcc30(pvVar5,puVar6,2,2,0,0,(undefined4 *)(iVar1 + -4),0x12);
  SelectPalette(pHVar2,DAT_006be5a0,0);
  FUN_00607710(param_1,pHVar2);
  if ((param_1[0x53] & 4U) == 0) {
    if ((param_1[0x53] & 0x40U) == 0) goto LAB_004a2b4c;
    pvVar5 = (void *)param_1[0x61];
  }
  else {
    pvVar5 = (void *)param_1[0x60];
  }
  FUN_005fc140(pvVar5,pHVar2,'@',0xcc0020,0x61,0x1d,0,0,0,0);
LAB_004a2b4c:
  if ((void *)param_1[0x5a] != (void *)0x0) {
    FUN_005fc140((void *)param_1[0x5a],pHVar2,'@',0xcc0020,100,0x2a,0,0,0,0);
  }
  if ((void *)param_1[0x49] != (void *)0x0) {
    FUN_00601ce0((void *)param_1[0x49],pHVar2);
  }
  if ((void *)param_1[100] != (void *)0x0) {
    FUN_00601ce0((void *)param_1[100],pHVar2);
  }
  EndPaint(hWnd,&tStack_40);
  return 0;
}

