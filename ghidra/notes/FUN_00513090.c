
void * __thiscall
FUN_00513090(void *this,undefined4 param_1,undefined4 param_2,undefined4 *param_3,undefined4 param_4
            )

{
  *(undefined4 *)((int)this + 4) = param_1;
  FUN_00520580((void *)((int)this + 8),param_3);
  *(undefined4 *)((int)this + 0x18) = param_2;
  *(undefined4 *)((int)this + 0x10) = param_4;
  *(undefined4 *)((int)this + 0x14) = 1;
  *(undefined4 *)((int)this + 0x1c) = 0;
  *(undefined ***)this = &PTR_FUN_0065e92c;
  return this;
}

