
void __thiscall FUN_00419870(void *this,undefined4 param_1,int param_2)

{
  if (param_2 == *(int *)((int)this + 0x144)) {
    *(uint *)((int)this + 4) = *(uint *)((int)this + 4) | 0x4000000;
  }
  FUN_0042e330((void *)((int)this + 0x138),param_2,*(int *)this);
  return;
}

