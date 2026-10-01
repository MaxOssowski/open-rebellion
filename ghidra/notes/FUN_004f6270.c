
void * __thiscall FUN_004f6270(void *this,void *param_1)

{
  if (*(int *)((int)this + 0x34) != 0) {
    FUN_005f2f90(param_1,*(int *)((int)this + 0x34));
    return param_1;
  }
  if (*(void **)((int)this + 0x2c) != (void *)0x0) {
    FUN_004486d0(*(void **)((int)this + 0x2c),param_1);
    return param_1;
  }
  FUN_005f35b0(param_1,&DAT_006b120c);
  return param_1;
}

