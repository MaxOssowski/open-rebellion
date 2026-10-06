
uint __cdecl FUN_0050f2e0(uint param_1,undefined4 *param_2)

{
  uint uVar1;
  
  *param_2 = 0;
  uVar1 = FUN_0050f0b0();
  if (uVar1 != 0) {
    switch(param_1 & 0xfffffffe) {
    case 4:
      *param_2 = &DAT_006b2b90;
      return uVar1;
    default:
      break;
    case 8:
      *param_2 = &DAT_006b2b88;
      return uVar1;
    case 0x10:
      *param_2 = &DAT_006b2ba8;
      return uVar1;
    case 0x40:
      *param_2 = &DAT_006b2b98;
      return uVar1;
    }
  }
  return 0;
}

