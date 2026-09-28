// FUN_0047f500

void __fastcall FUN_0047f500(void *param_1)

{
  short sVar1;
  
  *(uint *)((int)param_1 + 0x20) = *(uint *)((int)param_1 + 0x20) | 2;
  sVar1 = FUN_005f50e0((int)param_1 + 0x44);
  if (sVar1 != 0) {
    FUN_0047b9e0(param_1);
  }
  return;
}

