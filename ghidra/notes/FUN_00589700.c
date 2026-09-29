
undefined4 FUN_00589700(void)

{
  undefined4 uVar1;
  int *unaff_EBX;
  int unaff_ESI;
  undefined4 in_stack_00000010;
  void *in_stack_00000014;
  undefined4 in_stack_0000001c;
  int *in_stack_00000024;
  void *in_stack_00000028;
  undefined4 in_stack_00000030;
  
  uVar1 = in_stack_00000030;
  *unaff_EBX = 0;
  if (*(int *)((int)*(void **)(unaff_ESI + 4) + 0x2c) != 0) {
    if (*(int *)(unaff_ESI + 0xc) == 0) {
      *(undefined4 *)(unaff_ESI + 0xc) = 1;
      FUN_005887a0(*(void **)(unaff_ESI + 4),(int *)(unaff_ESI + 0x10),in_stack_00000030);
      FUN_005872a0(&stack0x00000008,*(undefined4 *)(unaff_ESI + 4));
      in_stack_0000001c = 0;
      FUN_00587b70(*(void **)(unaff_ESI + 4),(int *)&stack0x00000008,uVar1);
      *(undefined4 *)(unaff_ESI + 0x14) = in_stack_00000010;
      in_stack_0000001c = 0xffffffff;
      FUN_005872e0((undefined4 *)&stack0x00000008);
    }
    if (*unaff_EBX == 0) {
      FUN_00588a90(*(void **)(unaff_ESI + 4),in_stack_00000024,in_stack_00000028,
                   *(int *)(unaff_ESI + 0x10),*(int *)(unaff_ESI + 0x14),
                   (undefined4 *)(unaff_ESI + 8));
    }
  }
  *unaff_EBX = *(int *)(unaff_ESI + 8);
  ExceptionList = in_stack_00000014;
  return 1;
}

