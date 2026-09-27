
bool __thiscall FUN_00522a30(void *this,undefined4 param_1)

{
  int iVar1;
  bool bVar2;
  
  bVar2 = false;
  if (*(int *)((int)this + 0x68) == 0xb) {
    iVar1 = (**(code **)(*(int *)this + 0xac))(0x14,param_1);
    bVar2 = iVar1 != 0;
  }
  return bVar2;
}

