
void __thiscall FUN_00608280(void *this,undefined4 param_1)

{
  if (((*(byte *)((int)this + 0xf4) & 8) == 0) &&
     (*(undefined4 **)((int)this + 0xa0) != (undefined4 *)0x0)) {
    (**(code **)**(undefined4 **)((int)this + 0xa0))(1);
  }
  *(uint *)((int)this + 0xf4) = *(uint *)((int)this + 0xf4) | 8;
  *(undefined4 *)((int)this + 0xa0) = param_1;
  return;
}

