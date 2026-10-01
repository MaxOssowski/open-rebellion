
uint * __thiscall FUN_004025b0(void *this,uint *param_1)

{
  uint uVar1;
  int iVar2;
  uint uVar3;
  
  uVar1 = *(uint *)((int)this + 0x18);
  iVar2 = (**(code **)(*(int *)this + 4))();
  uVar3 = *param_1 & 0xffffff | iVar2 << 0x18;
  *param_1 = (uVar1 ^ uVar3) & 0xffffff ^ uVar3;
  return param_1;
}

