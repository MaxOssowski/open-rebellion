
int __thiscall
FUN_0056cdb0(void *param_1,int *param_2,undefined4 param_3,undefined4 *param_4,void *param_5)

{
  int *this;
  void *pvVar1;
  bool bVar2;
  int iVar3;
  int iVar4;
  undefined3 extraout_var;
  int iStack_10;
  undefined4 uStack_c;
  int iStack_8;
  int iStack_4;
  
  *param_4 = 0;
  iStack_10 = 0;
  iVar3 = FUN_00586720(param_1,&iStack_10);
  param_4 = (undefined4 *)0x0;
  iVar4 = FUN_0056c0e0(param_1,&param_4);
  if ((iVar4 == 0) || (iVar3 == 0)) {
    iVar3 = 0;
  }
  else {
    iVar3 = 1;
  }
  if (iStack_10 != 0) {
    uStack_c = 0;
    iVar4 = FUN_005337f0(param_2,(int)param_4,&uStack_c);
    if ((iVar4 == 0) || (iVar3 == 0)) {
      bVar2 = false;
    }
    else {
      bVar2 = true;
    }
    iVar3 = FUN_0055c6f0(uStack_c,&iStack_8);
    if ((iVar3 == 0) || (!bVar2)) {
      iVar3 = 0;
    }
    else {
      iVar3 = 1;
    }
    bVar2 = FUN_0053e2f0(iStack_8);
    if (CONCAT31(extraout_var,bVar2) != 0) {
      iVar4 = FUN_0055c9f0(&iStack_4);
      pvVar1 = param_5;
      if ((iVar4 == 0) || (iVar3 == 0)) {
        iVar3 = 0;
      }
      else {
        iVar3 = 1;
      }
      if (iStack_4 != 0) {
        iVar4 = FUN_0056c240(param_1,*(int *)((int)param_1 + 0xa8) + iStack_4,param_5);
        this = param_2;
        if ((iVar4 == 0) || (iVar3 == 0)) {
          iVar3 = 0;
        }
        else {
          iVar3 = 1;
        }
        iVar4 = (**(code **)(*param_2 + 0x1d8))();
        if (iVar4 != 0) {
          param_2 = (int *)0x0;
          iVar4 = FUN_00533850(this,(int)param_4,(int *)&param_2);
          if ((iVar4 == 0) || (iVar3 == 0)) {
            bVar2 = false;
          }
          else {
            bVar2 = true;
          }
          iVar3 = FUN_00534990(this,(int)param_4,(int)param_2 + DAT_006bb568,pvVar1);
          if ((iVar3 != 0) && (bVar2)) {
            return 1;
          }
          iVar3 = 0;
        }
      }
    }
  }
  return iVar3;
}

