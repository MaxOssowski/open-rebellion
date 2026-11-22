// Thunk; decompiled 2026-11-22 with Ghidra 12.1.3 headless (DecompileTargets.java,
// read-only project). The typed corpus leaves thunks out. Callers:
//   Class_006584c0_vt18 00403200
//   Class_00658520_vt1c 00404440
//   Class_006588a8_vt1c 00412c40
//   Class_00658bf8_vt14 00422ce0
//   Class_00658bf8_vt14 00422ce0
//   Class_00658bf8_vt14 00422ce0
//   Class_00658bf8_vt14 00422ce0
//   Class_00658bf8_vt14 00422ce0

int __fastcall thunk_FUN_005f5060(int param_1)

{
  int iVar1;
  int iVar2;
  
  iVar1 = *(int *)(param_1 + 4);
  iVar2 = 0;
  if (iVar1 != 0) {
    iVar2 = iVar1;
    for (iVar1 = *(int *)(iVar1 + 4); iVar1 != 0; iVar1 = *(int *)(iVar1 + 4)) {
      iVar2 = iVar1;
    }
  }
  return iVar2;
}

