
int * __fastcall FUN_004a25c0(int *param_1)

{
  int iVar1;
  int *piVar2;
  uint *puVar3;
  
  puVar3 = (uint *)(param_1 + 0x51);
  iVar1 = FUN_00401060();
  piVar2 = FUN_004f3220(iVar1,puVar3);
  if (piVar2 == (int *)0x0) {
    (**(code **)(*param_1 + 0x30))();
  }
  return piVar2;
}

