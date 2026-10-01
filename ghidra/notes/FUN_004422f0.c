
void * __thiscall
FUN_004422f0(void *this,undefined4 param_1,undefined4 param_2,undefined4 param_3,int param_4,
            int param_5,undefined4 param_6,int param_7,undefined4 param_8,undefined4 param_9,
            undefined4 param_10)

{
  FUN_004aa960(this,*(undefined4 *)(param_7 + 0x1c),param_2,param_3,param_4,param_5,param_7,param_9,
               param_10);
  *(undefined4 *)((int)this + 0x14c) = param_6;
  *(undefined ***)this = &PTR_FUN_00659bb8;
  *(undefined4 *)((int)this + 0x150) = param_8;
  return this;
}

