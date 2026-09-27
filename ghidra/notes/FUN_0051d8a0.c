
void __thiscall FUN_0051d8a0(void *this,int param_1,int param_2)

{
  if (param_1 != 0) {
    *(int *)(param_1 + 0x40) = *(int *)((int)this + 0x10) + param_2;
    if (*(void **)((int)this + 0xa8) != (void *)0x0) {
      FUN_0054ee30(*(void **)((int)this + 0xa8),param_1);
    }
  }
  return;
}

