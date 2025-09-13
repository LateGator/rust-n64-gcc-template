.set noreorder
.set noat
.set mips64

.global _start
.global _tlb_exception
.global _gen_exception
.global _exit
.global _boot_memsize
.global _boot_tvtype
.global _boot_consoletype

.section .vec.tlb, "ax"
_tlb_exception:
2:  b       2b
     nop

.section .vec.gen, "ax"
_gen_exception:
2:  b       2b
     nop

.section .boot, "x"
_start:
    li      $at, 0x24000000    // set CU1 and FR
    mtc0    $at, $12
    li      $at, 0x01000000    // set denorm flush
    ctc1    $at, $31
    la      $gp, _gp
    lui     $v1, 0xA400        // retreive boot params from DMEM
    lw      $at, 0($v1)
    lbu     $v0, 9($v1)
    lbu     $v1, 11($v1)
    sw      $at, %gp_rel(_boot_memsize)($gp)
    sb      $v0, %gp_rel(_boot_tvtype)($gp)
    jal     {main}
     sb     $v1, %gp_rel(_boot_consoletype)($gp)
_exit:
    b       _exit
     nop

.section .sbss._boot_memsize, "aw"
.balign  4
.type   _boot_memsize, @object
.size   _boot_memsize, 4
_boot_memsize: .space 4

.section .sbss._boot_tvtype, "aw"
.type   _boot_tvtype, @object
.size   _boot_tvtype, 1
_boot_tvtype: .space 1

.section .sbss._boot_consoletype, "aw"
.type   _boot_consoletype, @object
.size   _boot_consoletype, 1
_boot_consoletype: .space 1
