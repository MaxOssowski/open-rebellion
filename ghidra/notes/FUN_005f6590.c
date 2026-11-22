// Thunk; decompiled 2026-11-22 with Ghidra 12.1.3 headless (DecompileTargets.java,
// read-only project). The typed corpus leaves thunks out. Callers:
//   Class_0065a218_vt38 0046a9c0
//   Class_0066cbe8_vt4 005ec740
//   Class_0066cbe8_vt4 005ec740
//   Class_0066d118_vt4 005ecb90
//   Class_0066d118_vt4 005ecb90
//   FUN_00402720 00402720
//   FUN_00422620 00422620
//   FUN_0042a320 0042a320

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

