// Thunk; decompiled 2026-11-22 with Ghidra 12.1.3 headless (DecompileTargets.java,
// read-only project). The typed corpus leaves thunks out. No direct callers (reached through a vtable).

undefined4 __thiscall CapShip::thunk_FUN_00500550(CapShip *this)

{
  if (this->ClassRecord == 0) {
    return 0;
  }
  return *(undefined4 *)(this->ClassRecord + 0xc0);
}

