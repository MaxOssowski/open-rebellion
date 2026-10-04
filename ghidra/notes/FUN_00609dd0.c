
void __thiscall FUN_00609dd0(void *this,undefined4 *param_1,undefined4 param_2)

{
  undefined4 local_4;
  
  local_4._0_2_ = (short)param_2;
  if (*(int *)((int)this + 0xf0) != 0) {
    local_4 = CONCAT22(param_2._2_2_ - *(short *)((int)this + 0xc4),(short)local_4);
    *param_1 = local_4;
    return;
  }
  local_4 = CONCAT22(param_2._2_2_,(short)local_4 - *(short *)((int)this + 0xc4));
  *param_1 = local_4;
  return;
}

