
/* WARNING: Removing unreachable block (ram,0x0054020b) */

void __thiscall FUN_00540200(void *this,uint param_1)

{
  uint uVar1;
  
  uVar1 = *(uint *)((int)this + 4);
  if ((uVar1 & 1) != param_1) {
    *(uint *)((int)this + 4) = (param_1 ^ uVar1) & 1 ^ uVar1;
    *(int *)this = *(int *)this + 1;
  }
  return;
}

