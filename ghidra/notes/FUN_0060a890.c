
bool __thiscall FUN_0060a890(int param_1,int param_2,int param_3)

{
  bool bVar1;
  byte *pbVar2;
  byte *pbVar3;
  uint uVar4;
  
  uVar4 = *(uint *)(param_1 + 0xc);
  bVar1 = false;
  if ((uVar4 & 1) != 0) {
    return *(uint *)(param_2 + 0xc) < *(uint *)(param_3 + 0xc);
  }
  if ((uVar4 & 2) != 0) {
    pbVar2 = (byte *)FUN_00583c40(param_2 + 0x14);
    pbVar3 = (byte *)FUN_00583c40(param_3 + 0x14);
    uVar4 = FUN_00626ad0(pbVar2,pbVar3);
    return (int)uVar4 < 0;
  }
  if ((uVar4 & 8) != 0) {
    bVar1 = *(int *)(param_2 + 0x54) < *(int *)(param_3 + 0x54);
  }
  return bVar1;
}

