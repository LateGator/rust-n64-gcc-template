ENTRY(_start)

PHDRS {
    irq PT_LOAD AT ( 0x80000000 );
    main PT_LOAD AT ( 0x80000400 );
}

SECTIONS {
    .boot 0x80000000 : {
        KEEP(*(.vec.tlb))
        . = 0x180;
        KEEP(*(.vec.gen))
        . = ALIGN(32);
    } :irq

    .text 0x80000400 : {
        KEEP(*(.boot))
        . = ALIGN(32);
        *(.text .text.*)
        . = ALIGN(32);
    } :main

    .rodata : {
        *(.rodata .rodata.*)
        . = ALIGN(16);
    } :main

    .data : {
        *(.data .data.*)
        . = ALIGN(16);
        _gp = . + 0x8000;
        *(.sdata .sdata.*)
    } :main

    __rom_end = .;

    .bss : {
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        . = ALIGN(16);
    } :main

    __bss_end = .;

    /DISCARD/ : {
        *(.MIPS.*)
        *(.comment)
        *(.mdebug.*)
        *(.pdr)
        *(.reginfo)
        *(.got)
    }
}
