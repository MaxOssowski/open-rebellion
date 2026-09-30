
void __thiscall FUN_00601c60(void *this,uint param_1)

{
  undefined4 uVar1;
  
  *(uint *)((int)this + 0x24) = param_1;
  uVar1 = FUN_006002b0(param_1);
  *(undefined4 *)((int)this + 0x28) = uVar1;
  if (((byte)*(undefined4 *)((int)this + 0x2c) & 1 | 2) == 0) {
    *(undefined4 *)((int)this + 0x1c) = 0;
    *(undefined4 *)((int)this + 0x20) = 0;
  }
  return;
}

