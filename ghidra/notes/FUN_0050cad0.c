
void __thiscall FUN_0050cad0(void *this,void *param_1)

{
  if ((((*(uint *)((int)this + 0x88) >> 8 & 1) != 0) &&
      (((byte)*(uint *)((int)this + 0x24) & 0xc0) == 0x80)) &&
     ((*(uint *)((int)this + 0x88) & 4) == 0)) {
    FUN_0050c9f0(this,DAT_006bb430,*(uint *)((int)this + 0x24) >> 6 & 3,param_1);
  }
  return;
}

