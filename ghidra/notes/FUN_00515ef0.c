
bool __cdecl FUN_00515ef0(uint *param_1,int param_2,void *param_3)

{
  void *this;
  int iVar1;
  bool bVar2;
  
  this = (void *)FUN_00505f60(param_1);
  bVar2 = this != (void *)0x0;
  if ((this != (void *)0x0) && (param_2 != 0)) {
    iVar1 = FUN_0050b8e0(this,param_3);
    if ((iVar1 != 0) && (bVar2)) {
      return true;
    }
    bVar2 = false;
  }
  return bVar2;
}

