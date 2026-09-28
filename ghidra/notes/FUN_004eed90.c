// FUN_004eed90

void __thiscall FUN_004eed90(void *this,void *param_1)

{
  int iVar1;
  uint local_4;
  
  local_4 = (uint)*(short *)((int)this + 0x62);
  iVar1 = FUN_0053e900(local_4,(int)*(short *)((int)this + 0x8c),(int)*(short *)((int)this + 0x94),
                       &local_4);
  if (iVar1 != 0) {
    FUN_004edf30(this,local_4,param_1);
  }
  return;
}

