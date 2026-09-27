
void __thiscall FUN_00540230(void *this,uint param_1)

{
  if ((int)DAT_00661a90 < (int)param_1) {
    param_1 = DAT_00661a90;
  }
  if (*(uint *)((int)this + 4) >> 0x10 != param_1) {
    *(uint *)((int)this + 4) = *(uint *)((int)this + 4) & 0xffff | param_1 << 0x10;
    *(int *)this = *(int *)this + 1;
    if ((int)param_1 < 0) {
      FUN_00540200(this,0);
    }
  }
  return;
}

