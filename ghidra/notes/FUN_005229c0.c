
bool __thiscall FUN_005229c0(void *this,void *param_1)

{
  bool bVar1;
  undefined3 extraout_var;
  uint uVar2;
  bool bVar3;
  
  bVar3 = true;
  bVar1 = FUN_00520d60(this);
  if ((CONCAT31(extraout_var,bVar1) == 0) && (*(int *)((int)this + 100) != 0)) {
    uVar2 = FUN_0053fa60(0x38c,1,this,(int *)(*(int *)((int)this + 0x54) + 0x28),param_1);
    bVar3 = uVar2 != 0;
  }
  return bVar3;
}

