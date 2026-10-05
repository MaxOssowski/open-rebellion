
void __thiscall FUN_0041c680(void *this,HMODULE param_1,uint param_2)

{
  HRSRC hResInfo;
  HGLOBAL pvVar1;
  
  FUN_0041c8f0((int)this);
  hResInfo = FindResourceA(param_1,(LPCSTR)(param_2 & 0xffff),(LPCSTR)0x12e);
  if (hResInfo != (HRSRC)0x0) {
    pvVar1 = LoadResource(param_1,hResInfo);
    if (pvVar1 != (HGLOBAL)0x0) {
      FUN_0041c6c0(this,pvVar1);
    }
  }
  return;
}

