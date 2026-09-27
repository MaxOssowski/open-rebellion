
void __thiscall FUN_00540270(void *this,uint param_1)

{
  if ((int)DAT_00661a8c < (int)param_1) {
    param_1 = DAT_00661a8c;
  }
  if ((*(uint *)((int)this + 4) >> 1 & 0x7fff) != param_1) {
    *(uint *)((int)this + 4) = *(uint *)((int)this + 4) & 0xffff0001 | (param_1 & 0x7fff) << 1;
    *(int *)this = *(int *)this + 1;
  }
  return;
}

