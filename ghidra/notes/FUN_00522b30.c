
undefined4 __thiscall FUN_00522b30(void *param_1,uint *param_2,int param_3,void *param_4)

{
  void *pvVar1;
  bool bVar2;
  undefined3 extraout_var;
  int iVar3;
  int *this;
  uint *puVar4;
  int *piVar5;
  int iVar6;
  uint uVar7;
  undefined4 uVar8;
  bool bVar9;
  void *pvVar10;
  int iStack_48;
  int aiStack_44 [13];
  int *piStack_10;
  void *pvStack_c;
  undefined1 *puStack_8;
  undefined4 uStack_4;
  
  uStack_4 = 0xffffffff;
  puStack_8 = &LAB_00643a60;
  pvStack_c = ExceptionList;
  ExceptionList = &pvStack_c;
  bVar2 = FUN_0053a000((int)param_1);
  if (CONCAT31(extraout_var,bVar2) == 0) {
    bVar2 = false;
  }
  else {
    aiStack_44[0] = 0x30;
    aiStack_44[1] = 0x40;
    if ((*param_2 >> 0x18 < 0x30) || (0x3f < *param_2 >> 0x18)) {
      bVar2 = false;
      FUN_00619730();
    }
    else {
      bVar2 = true;
      FUN_00619730();
    }
  }
  bVar9 = false;
  if (bVar2) {
    iVar3 = FUN_00520c30(param_2);
    bVar9 = iVar3 == 0;
  }
  uVar8 = 0;
  if (bVar9) {
    this = (int *)FUN_00505d40(param_2);
    uVar8 = 0;
    if ((this != (int *)0x0) &&
       (uVar8 = 0, (((byte)*(undefined4 *)((int)param_1 + 0x24) ^ (byte)this[9]) & 0xc0) == 0)) {
      iVar3 = (**(code **)(*this + 0x1d4))();
      if ((iVar3 == 0) || (param_3 == 0)) {
        bVar2 = true;
      }
      else {
        bVar2 = false;
      }
      uVar8 = 0;
      if (bVar2) {
        puVar4 = FUN_0042d170(this,aiStack_44);
        uStack_4 = 0;
        iVar3 = FUN_004ece60(puVar4);
        uStack_4 = 0xffffffff;
        FUN_00619730();
        uVar8 = 0;
        if (iVar3 == 0) {
          FUN_00525bb0(aiStack_44 + 2,param_1);
          uStack_4 = 1;
          FUN_00525930((int)(aiStack_44 + 2));
          bVar2 = true;
          if (piStack_10 != (int *)0x0) {
            FUN_004ece30(aiStack_44);
            uStack_4._0_1_ = 2;
            FUN_004ece30(&iStack_48);
            uStack_4 = CONCAT31(uStack_4._1_3_,3);
            iVar3 = (**(code **)(*this + 0xc))(aiStack_44);
            bVar2 = iVar3 == 0;
            uVar7 = 0;
            if (!bVar2) {
              iVar3 = (**(code **)(*piStack_10 + 0xc))(&iStack_48);
              bVar2 = iVar3 == 0;
              uVar7 = 0;
              if (!bVar2) {
                uVar7 = (uint)(iStack_48 == aiStack_44[0]);
                bVar2 = uVar7 == 0;
              }
            }
            if (!bVar2) {
              uVar7 = (uint)(this[0x14] ^ ~piStack_10[0x14]) >> 5 & 1;
            }
            bVar2 = false;
            if ((uVar7 != 0) &&
               (bVar2 = false, ((uint)(this[0x14] ^ ~piStack_10[0x14]) >> 0xb & 1) != 0)) {
              bVar2 = piStack_10[0x11] == this[0x11];
            }
            uStack_4._0_1_ = 2;
            FUN_00619730();
            uStack_4 = CONCAT31(uStack_4._1_3_,1);
            FUN_00619730();
          }
          uStack_4 = 0xffffffff;
          FUN_00525c50(aiStack_44 + 2);
          pvVar1 = param_4;
          uVar8 = 0;
          if (bVar2) {
            FUN_004fd450(aiStack_44 + 2,(int)param_4);
            uStack_4 = 4;
            FUN_004fd620(aiStack_44 + 2,1,1);
            FUN_004fd620(aiStack_44 + 2,2,1);
            puVar4 = FUN_004025b0(param_1,(uint *)&param_4);
            uStack_4._0_1_ = 5;
            iVar3 = FUN_00534230(this,(int *)puVar4,aiStack_44 + 2);
            uStack_4._0_1_ = 4;
            FUN_00619730();
            piVar5 = FUN_00402d80(param_1,&param_4);
            uStack_4._0_1_ = 6;
            iVar6 = FUN_005342e0(this,piVar5,pvVar1);
            if ((iVar6 == 0) || (iVar3 == 0)) {
              bVar2 = false;
            }
            else {
              bVar2 = true;
            }
            uStack_4 = CONCAT31(uStack_4._1_3_,4);
            FUN_00619730();
            iVar3 = FUN_00520b70((int)param_1);
            if (iVar3 != 0) {
              iVar3 = FUN_00534870(this,1,pvVar1);
              if ((iVar3 == 0) || (!bVar2)) {
                bVar2 = false;
              }
              else {
                bVar2 = true;
              }
            }
            iVar3 = FUN_005348e0(this,*(uint *)((int)param_1 + 0xa4) >> 2 & 1,pvVar1);
            if ((iVar3 == 0) || (!bVar2)) {
              bVar2 = false;
            }
            else {
              bVar2 = true;
            }
            iVar3 = FUN_005344f0(this,param_3,pvVar1);
            if ((iVar3 == 0) || (!bVar2)) {
              bVar2 = false;
            }
            else {
              bVar2 = true;
            }
            pvVar10 = pvVar1;
            iVar3 = FUN_00520b90((int)param_1);
            iVar3 = FUN_005346b0(this,(uint)(iVar3 != 0),pvVar10);
            if ((iVar3 == 0) || (!bVar2)) {
              bVar2 = false;
            }
            else {
              bVar2 = true;
            }
            if (param_3 == 0) {
              iVar3 = (**(code **)(*this + 0x1d4))();
              if (iVar3 == 0) {
                FUN_004f4390(aiStack_44,(int)param_1 + 0x84);
                uStack_4._0_1_ = 9;
                FUN_004f44b0(aiStack_44,param_2,0);
                iVar3 = FUN_00521e30(param_1,aiStack_44,pvVar1);
                if ((iVar3 == 0) || (!bVar2)) {
                  uVar8 = 0;
                }
                else {
                  uVar8 = 1;
                }
                uStack_4 = CONCAT31(uStack_4._1_3_,4);
              }
              else {
                FUN_004f4390(aiStack_44,(int)param_1 + 0x94);
                uStack_4._0_1_ = 8;
                FUN_004f44b0(aiStack_44,param_2,0);
                iVar3 = FUN_00521fb0(param_1,aiStack_44,pvVar1);
                if ((iVar3 == 0) || (!bVar2)) {
                  uVar8 = 0;
                  uStack_4 = CONCAT31(uStack_4._1_3_,4);
                }
                else {
                  uVar8 = 1;
                  uStack_4 = CONCAT31(uStack_4._1_3_,4);
                }
              }
            }
            else {
              FUN_004f4390(aiStack_44,(int)param_1 + 0x8c);
              uStack_4._0_1_ = 7;
              FUN_004f44b0(aiStack_44,param_2,0);
              iVar3 = FUN_00521ef0(param_1,aiStack_44,pvVar1);
              if ((iVar3 == 0) || (!bVar2)) {
                uVar8 = 0;
                uStack_4 = CONCAT31(uStack_4._1_3_,4);
              }
              else {
                uVar8 = 1;
                uStack_4 = CONCAT31(uStack_4._1_3_,4);
              }
            }
            FUN_004f4380(aiStack_44);
            uStack_4 = 0xffffffff;
            FUN_004fd4d0(aiStack_44 + 2);
          }
        }
      }
    }
  }
  ExceptionList = pvStack_c;
  return uVar8;
}

