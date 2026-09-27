
void __thiscall FUN_0050cb20(void *this,void *param_1)

{
  uint uVar1;
  int iVar2;
  uint local_8;
  uint local_4;
  
  uVar1 = 0;
  if ((*(uint *)((int)this + 0x88) >> 5 & 1) != 0) {
    uVar1 = FUN_00509980(this,&local_8);
  }
  if (uVar1 != 0) {
    uVar1 = FUN_00509890(this,&local_4);
    if (uVar1 != 0) {
      iVar2 = DAT_006bb48c;
      if (local_8 == 0) {
        iVar2 = DAT_006bb494;
      }
      FUN_0050c9f0(this,iVar2,local_4,param_1);
    }
  }
  return;
}

