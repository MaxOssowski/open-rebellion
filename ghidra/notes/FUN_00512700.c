
uint __thiscall FUN_00512700(void *param_1,int *param_2,uint param_3,int *param_4)

{
  int *this;
  bool bVar1;
  uint *puVar2;
  void *pvVar3;
  int iVar4;
  uint uVar5;
  uint uVar6;
  undefined4 *puVar7;
  undefined4 auStack_2c [7];
  int iStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  this = param_4;
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_00642208;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  (**(code **)(*param_4 + 4))();
  if (*param_2 == 1) {
    uVar6 = 1;
  }
  else if (*param_2 == 2) {
    uVar6 = 2;
  }
  else {
    uVar6 = 0;
  }
  uVar5 = (uint)(uVar6 != 0);
  if ((param_3 & 0xfffffffe) == 0) {
    puVar2 = FUN_004025b0(param_1,(uint *)&param_4);
    uStack_4 = 0;
    bVar1 = FUN_004f44b0(this,puVar2,0);
    uVar5 = (uint)bVar1;
    uStack_4 = 0xffffffff;
    FUN_00619730();
  }
  else {
    switch(param_3 & 0xfffffffe) {
    case 4:
      iVar4 = param_2[3];
      if (iVar4 == 0x200) {
        if (uVar5 != 0) {
          FUN_0053b6e0(auStack_2c,param_1,3);
          uStack_4 = 1;
          uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
          uStack_4 = 0xffffffff;
          FUN_0053b7e0(auStack_2c);
        }
      }
      else if (iVar4 == 0x214) {
        if (uVar5 != 0) {
          FUN_0052c170(auStack_2c,param_1,3);
          uStack_4 = 2;
          uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
          uStack_4 = 0xffffffff;
          FUN_0052c1f0(auStack_2c);
        }
      }
      else if ((iVar4 == 0x216) && (uVar5 != 0)) {
        FUN_0052c170(auStack_2c,param_1,3);
        uStack_4 = 3;
        uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
        uStack_4 = 0xffffffff;
        FUN_0052c1f0(auStack_2c);
      }
      break;
    default:
      uVar5 = 0;
      break;
    case 8:
      switch(param_2[3]) {
      case 0x200:
        if (uVar5 != 0) {
          FUN_00504c40(auStack_2c,param_1,3);
          uStack_4 = 7;
          iVar4 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
          uStack_4 = 0xffffffff;
          FUN_00504d40(auStack_2c);
          uVar5 = 0;
          if (iVar4 != 0) {
            FUN_00503a50(auStack_2c,param_1,3,uVar6);
            uStack_4 = 8;
            iVar4 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
            uStack_4 = 0xffffffff;
            FUN_00503ad0(auStack_2c);
            uVar5 = 0;
            if (iVar4 != 0) {
              FUN_00527050(auStack_2c,param_1,3,uVar6);
              uStack_4 = 9;
              uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
              uStack_4 = 0xffffffff;
              FUN_005270d0(auStack_2c);
            }
          }
        }
        break;
      case 0x201:
      case 0x202:
        if (uVar5 != 0) {
          FUN_00504c40(auStack_2c,param_1,3);
          uStack_4 = 4;
          iVar4 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
          uStack_4 = 0xffffffff;
          FUN_00504d40(auStack_2c);
          uVar5 = 0;
          if (iVar4 != 0) {
            FUN_005039d0(auStack_2c,param_1,3);
            uStack_4 = 5;
            iVar4 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
            uStack_4 = 0xffffffff;
            FUN_00503ad0(auStack_2c);
            uVar5 = 0;
            if (iVar4 != 0) {
              FUN_00536da0(auStack_2c,param_1,3);
              uStack_4 = 6;
              uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
              uStack_4 = 0xffffffff;
              FUN_00536ea0(auStack_2c);
            }
          }
        }
        break;
      case 0x240:
        if (uVar5 != 0) {
          FUN_00536da0(auStack_2c,param_1,3);
          uStack_4 = 0xb;
          uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
          uStack_4 = 0xffffffff;
          FUN_00536ea0(auStack_2c);
        }
        break;
      case 0x242:
        if (uVar5 != 0) {
          FUN_00536da0(auStack_2c,param_1,3);
          uStack_4 = 10;
          uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
          uStack_4 = 0xffffffff;
          FUN_00536ea0(auStack_2c);
        }
      }
      break;
    case 0x10:
      if (uVar5 != 0) {
        FUN_004ffe70(auStack_2c,param_1,3);
        uStack_4 = 0xc;
        uVar5 = FUN_00553350((uint)auStack_2c,this,1,uVar6);
        uStack_4 = 0xffffffff;
        FUN_004fff70(auStack_2c);
      }
      break;
    case 0x40:
      if (uVar5 != 0) {
        FUN_00536e20(auStack_2c,param_1,3,uVar6);
        uStack_4 = 0xd;
        FUN_00513120((int)auStack_2c);
        while (iStack_10 != 0) {
          puVar7 = &param_4;
          pvVar3 = (void *)FUN_0052bed0((int)auStack_2c);
          FUN_0042d170(pvVar3,puVar7);
          uStack_4 = CONCAT31(uStack_4._1_3_,0xe);
          iVar4 = FUN_004ece60((uint *)&param_4);
          if (((iVar4 != 0) &&
              (iVar4 = FUN_0052bed0((int)auStack_2c), (*(uint *)(iVar4 + 0x78) >> 8 & 1) == 0)) &&
             (pvVar3 = FUN_004f5940(this,(uint *)&param_4), pvVar3 == (void *)0x0)) {
            FUN_004f44b0(this,&param_4,0);
          }
          uStack_4 = CONCAT31(uStack_4._1_3_,0xd);
          FUN_00619730();
          FUN_005130d0((int)auStack_2c);
        }
        uStack_4 = 0xffffffff;
        FUN_00536ea0(auStack_2c);
      }
    }
  }
  ExceptionList = pvStack_c;
  return uVar5;
}

