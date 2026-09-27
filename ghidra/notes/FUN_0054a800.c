
undefined4 __thiscall FUN_0054a800(void *this,int param_1,void *param_2)

{
  bool bVar1;
  undefined4 uVar2;
  undefined3 extraout_var;
  uint uVar3;
  
  uVar2 = 1;
  if (param_1 != 0) {
    bVar1 = FUN_00544030(this,param_2);
    uVar3 = FUN_0053fa60(0x392,0,this,(int *)((int)this + 0x5c),param_2);
    if ((uVar3 != 0) && (CONCAT31(extraout_var,bVar1) != 0)) {
      return 1;
    }
    uVar2 = 0;
  }
  return uVar2;
}

