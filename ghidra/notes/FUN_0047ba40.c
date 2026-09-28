// FUN_0047ba40

undefined4 __thiscall FUN_0047ba40(void *this,uint *param_1)

{
  void *pvVar1;
  int iVar2;
  undefined4 uVar3;
  
  uVar3 = 0;
  pvVar1 = FUN_004f5940(*(void **)((int)this + 100),param_1);
  if (((pvVar1 != (void *)0x0) && ((*(uint *)((int)pvVar1 + 0x30) & 0x7400051) == 0)) &&
     (*(int *)((int)pvVar1 + 0x24) == *(int *)((int)this + 0x18))) {
    iVar2 = FUN_0047ba90(this,param_1);
    if (iVar2 != 0) {
      iVar2 = (**(code **)(*(int *)this + 0x3c))(param_1);
      if (iVar2 != 0) {
        uVar3 = 1;
      }
    }
  }
  return uVar3;
}

