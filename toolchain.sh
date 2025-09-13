#!/bin/bash
# N64 MIPS GCC toolchain build/install script for Unix distributions
# Original script (c) 2012-2023 DragonMinded and libDragon Contributors.

# Bash strict mode http://redsymbol.net/articles/unofficial-bash-strict-mode/
set -euo pipefail
IFS=$'\n\t'

SCRIPT_NAME=$0
SCRIPT_DIR=$(dirname $(realpath $0))

perror () { echo "$@" 1>&2; }

mode_help () {
  "$1" "usage: $SCRIPT_NAME"$' COMMAND [TARGET ...]
Supported commands:    fetch update configure make install clean help
Supported targets:     all binutils gcc gccjit newlib rustc_codegen_gcc libdragon

Supported environment variables:
  N64_INST             installation prefix (required for configure)
  BUILD_PATH           base build directory (default: '"$SCRIPT_DIR"$'/n64-toolchain)
  JOBS                 number of simultaneous make commands passed to `make -j`
  GCC_SOURCE_DIR       custom source directory for gcc
  RCGCC_SOURCE_DIR     custom source directory for rustc_codegen_gcc
  BINUTILS_SOURCE_DIR  custom source directory for binutils
  NEWLIB_SOURCE_DIR    custom source directory for newlib
  MAKE_SOURCE_DIR      custom source directory for make
  LIBDRAGON_SOURCE_DIR custom source directory for libdragon'
}

# Targets that will be matched when using the `all` target.
ALL_TARGETS=(binutils gcc gccjit newlib rustc_codegen_gcc)

# Path where the toolchain will be built.
BUILD_PATH="${BUILD_PATH:-$SCRIPT_DIR/n64-toolchain}"

# Set N64_INST before calling the script to change the default installation directory path
INSTALL_PATH="${N64_INST:-}"
# Set PATH for newlib to compile using GCC for MIPS N64 (pass 1)
export PATH="$PATH:$INSTALL_PATH/bin"

# Determine how many parallel Make jobs to run based on CPU count
JOBS="${JOBS:-$(getconf _NPROCESSORS_ONLN)}"
JOBS="${JOBS:-1}" # If getconf returned nothing, default to 1

CROSS_PREFIX=$INSTALL_PATH

# List of Rust multilib targets to build rustc_codegen_gcc for
readarray -td ' ' RUSTC_GCC_ABIS <<< "${RUSTC_GCC_ABIS:-"eabi32 eabi64 n32 o32 o64"}"

# Custom source directories to use, e.g. if building from git
BINUTILS_SOURCE_DIR=${BINUTILS_SOURCE_DIR:-""}
GCC_SOURCE_DIR=${GCC_SOURCE_DIR:-""}
RCGCC_SOURCE_DIR=${RCGCC_SOURCE_DIR:-""}
NEWLIB_SOURCE_DIR=${NEWLIB_SOURCE_DIR:-""}
MAKE_SOURCE_DIR=${MAKE_SOURCE_DIR:-""}
LIBDRAGON_SOURCE_DIR=${LIBDRAGON_SOURCE_DIR:-""}

# Dependency source libs (Versions)
NEWLIB_V=4.5.0.20241231
GMP_V=6.3.0
MPC_V=1.3.1
MPFR_V=4.2.1
MAKE_V=4.4.1

# Check if a command-line tool is available: status 0 means "yes"; status 1 means "no"
command_exists () {
    (command -v "$1" >/dev/null 2>&1)
    return $?
}

# Run a command and if it fails, try to run it again with elevated privileges
exec_priv () {
    $@ 2>/dev/null || sudo env PATH="$PATH" $@
}

# Download the file URL using wget or curl (depending on which is installed)
download () {
    if   command_exists wget ; then wget -c  "$1"
    elif command_exists curl ; then curl -LO "$1"
    else
        perror "Install wget or curl to download toolchain sources"
        return 1
    fi
}

# Create a rust target JSON for a specific abi
make_rust_target () {
    local layout oformat noredzone args
    case $1 in
        eabi32) args=(-mabi=eabi -mgp32);;
        eabi64) args=(-mabi=eabi -mgp64 -mlong32 -msym32);;
        n32)    args=(-mabi=n32);;
        o32)    args=(-mabi=32);;
        o64)    args=(-mabi=o64);;
        o64)    perror "Unknown ABI $1"; exit 1;;
    esac
    case $1 in
        o32 | eabi32)
          oformat=elf32-bigmips
          layout="E-m:e-p:32:32-i8:8:32-i16:16:32-i64:64-n32-S64"
        ;;
        eabi64 | o64)
          oformat=elf32-bigmips
          layout="E-m:e-p:32:32-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S64"
        ;;
        n32)
          oformat=elf32-nbigmips
          layout="E-m:e-p:32:32-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128"
        ;;
    esac
    case $1 in
        eabi32 | eabi64 | n32) noredzone=false;;
        o32 | o64)             noredzone=true;;
    esac
    local qargs=""
    for arg in ${args[@]}; do
        qargs+=${qargs:+,}'"'$arg'"';
    done
    cat <<- EOF
{
    "arch": "mips64",
    "cpu": "mips3",
    "data-layout": "$layout",
    "disable-redzone": $noredzone,
    "env": "unknown",
    "executables": true,
    "exe-suffix": ".elf",
    "features": "+mips3",
    "default-codegen-backend": "$N64_INST/lib/librustc_codegen_gcc.so",
    "no-default-libraries": false,
    "linker": "$N64_INST/bin/mips64vr-n64-elf-g++",
    "linker-flavor": "gnu-cc",
    "llvm-abiname": "n32",
    "llvm-target": "mips64-unknown-unknown",
    "max-atomic-width": 64,
    "os": "none",
    "panic-strategy": "abort",
    "asm-args": [ $qargs ],
    "llvm-args": [ $qargs ],
    "pre-link-args": {
        "gnu-cc": ["-Wl,--script=n64.ld","-Wl,--oformat=$oformat",$qargs]
    },
    "relocation-model": "static",
    "singlethread": true,
    "target-c-int-width": 32,
    "target-endian": "big",
    "target-pointer-width": "32",
    "vendor": "n64"
}
EOF
}

make_rust_target_files () {
    local jsonfile buildpath=$(realpath "$BUILD_PATH")
    for abi in ${RUSTC_GCC_ABIS[@]}; do
        jsonfile="$buildpath/mips64vr-n64-elf$abi.json"
        test -f "$jsonfile" || (make_rust_target $abi > "$jsonfile")
    done
}

# Dependency downloads and unpack
mode_fetch () {
    local gcc_src
    if [ -z "$GCC_SOURCE_DIR" ]; then
      gcc_src=gcc
    else
      gcc_src="$GCC_SOURCE_DIR"
    fi
    case $1 in
        binutils)
            test -d binutils-gdb                || git clone https://github.com/lategator/binutils-gdb.git master
        ;;
        gcc | gccjit)
            test -d gcc                         || git clone https://github.com/lategator/gcc.git master
        ;;
        rustc_codegen_gcc)
            test -d rustc_codegen_gcc           || git clone https://github.com/lategator/rustc_codegen_gcc.git master
        ;;
        libdragon)
            test -d libdragon                   || git clone https://github.com/DragonMinded/libdragon.git preview
        ;;
        newlib)
            test -f "newlib-$NEWLIB_V.tar.gz"   || download "https://sourceware.org/pub/newlib/newlib-$NEWLIB_V.tar.gz"
            test -d "newlib-$NEWLIB_V"          || tar -xzf "newlib-$NEWLIB_V.tar.gz"
        ;;
        gmp) if [ "$GMP_V" != "" ]; then
            test -f "gmp-$GMP_V.tar.bz2"        || download "https://ftp.gnu.org/gnu/gmp/gmp-$GMP_V.tar.bz2"
            tar -xjf "gmp-$GMP_V.tar.bz2"
            mkdir -p "$gcc_src"
            pushd "$gcc_src"
            ln -sf ../"gmp-$GMP_V" "gmp"
            popd
        fi ;;
        mpc) if [ "$MPC_V" != "" ]; then
            test -f "mpc-$MPC_V.tar.gz"         || download "https://ftp.gnu.org/gnu/mpc/mpc-$MPC_V.tar.gz"
            tar -xzf "mpc-$MPC_V.tar.gz"
            mkdir -p "$gcc_src"
            pushd "$gcc_src"
            ln -sf ../"mpc-$MPC_V" "mpc"
            popd
        fi ;;
        mpfr) if [ "$MPFR_V" != "" ]; then
            test -f "mpfr-$MPFR_V.tar.gz"       || download "https://ftp.gnu.org/gnu/mpfr/mpfr-$MPFR_V.tar.gz"
            tar -xzf "mpfr-$MPFR_V.tar.gz"
            mkdir -p "$gcc_src"
            pushd "$gcc_src"
            ln -sf ../"mpfr-$MPFR_V" "mpfr"
            popd
        fi ;;
        make) if [ "$MAKE_V" != "" ]; then
            test -f "make-$MAKE_V.tar.gz"       || download "https://ftp.gnu.org/gnu/make/make-$MAKE_V.tar.gz"
            tar -xzf "make-$MAKE_V.tar.gz"
        fi ;;
        *) perror "Unknown fetch target: $1"; return 1;;
    esac
}

mode_update () {
    case $1 in
        binutils) mode_fetch $1 && (cd binutils-gdb && git pull);;
        gcc | gccjit) mode_fetch $1 && (cd gcc && git pull);;
        rustc_codegen_gcc) mode_fetch $1 && (cd rustc_codegen_gcc && git pull);;
        *) perror "Unknown update target: $1"; return 1;;
    esac
}

repeat_mode () {
    local mode=mode_$1
    shift
    if [[ $# -eq 0 ]]; then mode_help perror; return 1; fi
    while [[ $# -gt 0 ]]; do
        if [ "$1" = all ]; then
            for tgt in ${ALL_TARGETS[@]}; do
                echo TGT $tgt
                "$mode" "$tgt"
            done
        else
            "$mode" "$1"
        fi
        shift
    done
}

mode_configure () {
    # Check that N64_INST is defined
    if [ -z "${N64_INST-}" ]; then
        perror "N64_INST environment variable is not defined."
        perror "Please define N64_INST and point it to the requested installation directory before running configure"
        exit 1
    fi
    # GCC configure arguments to use system GMP/MPC/MFPF
    GCC_CONFIGURE_ARGS=(
        "--prefix=$CROSS_PREFIX"
        "--target=mips64vr-n64-elf"
        "--with-as=$CROSS_PREFIX/bin/mips64vr-n64-elf-as"
        "--with-ld=$CROSS_PREFIX/bin/mips64vr-n64-elf-ld"
        "--with-gnu-as"
        "--with-gnu-ld"
        "--program-prefix=mips64vr-n64-elf-"
        "--with-arch=vr4300"
        "--with-tune=vr4300"
        "--without-headers"
        "--disable-libssp"
        "--enable-multilib"
        "--disable-shared"
        "--with-newlib"
        "--disable-win32-registry"
        "--disable-nls"
        "--disable-werror"
        #"--disable-threads"
    )
    # Compilation on macOS via homebrew
    if [[ $OSTYPE == 'darwin'* ]]; then
        if ! command_exists brew; then
            perror "Compilation on macOS is supported via Homebrew (https://brew.sh)"
            perror "Please install homebrew and try again"
            return 1
        fi
        brew install -q gsed gcc make texinfo zlib
        case $1 in
            gcc | gccjit) brew install -q gmp mpfr libmpc isl libpng lz4;;
        esac

        # Tell GCC configure where to find the dependent libraries
        GCC_CONFIGURE_ARGS+=(
            "--with-gmp=$(brew --prefix)"
            "--with-mpfr=$(brew --prefix)"
            "--with-mpc=$(brew --prefix)"
            "--with-zlib=$(brew --prefix)"
        )

        # Install GNU sed as default sed in PATH. GCC compilation fails otherwise,
        # because it does not work with BSD sed.
        PATH="$(brew --prefix gsed)/libexec/gnubin:$PATH"
        export PATH
    else
        GCC_CONFIGURE_ARGS+=("--with-system-zlib")
    fi

    export AR_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-ar"
    export AS_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-as"
    export LD_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-ld"
    export NM_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-nm"
    export OBJCOPY_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-objcopy"
    export OBJDUMP_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-objdump"
    export RANLIB_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-ranlib"
    export READELF_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-readelf"
    export STRIP_FOR_TARGET="$CROSS_PREFIX/bin/mips64vr-n64-elf-strip"

    case $1 in
        binutils)
            if [ -z "$BINUTILS_SOURCE_DIR" ]; then
                mode_fetch binutils
                binutils_src=../binutils-gdb
            else
                binutils_src=$BINUTILS_SOURCE_DIR
            fi
            mkdir -p binutils-build
            pushd binutils-build
            $binutils_src/configure \
                --prefix="$CROSS_PREFIX" \
                --target=mips64vr-n64-elf \
                --with-cpu=mips64vr4300 \
                --program-prefix=mips64vr-n64-elf- \
                --disable-werror
            popd
            ;;
        gcc)
            if [ -z "$GCC_SOURCE_DIR" ]; then
                repeat_mode fetch gcc gmp mpc mpfr
                gcc_src=../gcc
            else
                gcc_src=$GCC_SOURCE_DIR
            fi
            mkdir -p gcc-build
            pushd gcc-build
            target_configargs="--disable-hosted-libstdcxx" \
            $gcc_src/configure "${GCC_CONFIGURE_ARGS[@]}" \
                --enable-languages=c,c++
            popd
            ;;
        gccjit)
            if [ -z "$GCC_SOURCE_DIR" ]; then
                repeat_mode fetch gcc gmp mpc mpfr
                gcc_src=../gcc
            else
                gcc_src=$GCC_SOURCE_DIR
            fi
            mkdir -p gccjit-build
            pushd gccjit-build
            target_configargs="--disable-hosted-libstdcxx" \
            $gcc_src/configure "${GCC_CONFIGURE_ARGS[@]}" \
                --enable-languages=jit --enable-host-shared
            popd
            ;;
        rustc_codegen_gcc)
            if [ -z "$RCGCC_SOURCE_DIR" ]; then
                mode_fetch rustc_codegen_gcc
                rcgcc_src=rustc_codegen_gcc
                test -f "$rcgcc_src/config.toml" || echo 'gcc-path = "'"$N64_INST"'/lib"' > "$rcgcc_src/config.toml"
            else
                rcgcc_src=$RCGCC_SOURCE_DIR
            fi
            make_rust_target_files
            pushd "$rcgcc_src"
            ./y.sh prepare --cross --only-libcore
            popd
            ;;
        newlib)
            if [ -z "$NEWLIB_SOURCE_DIR" ]; then
                mode_fetch newlib
                newlib_src=../"newlib-$NEWLIB_V"
            else
                newlib_src=$NEWLIB_SOURCE_DIR
            fi
            mkdir -p newlib-build
            pushd newlib-build
            CFLAGS_FOR_TARGET="-DHAVE_ASSERT_FUNC -O2 -fpermissive" $newlib_src/configure \
                --prefix="$CROSS_PREFIX" \
                --target=mips64vr-n64-elf \
                --with-cpu=mips64vr4300 \
                --disable-libssp \
                --disable-werror \
                --enable-newlib-multithread \
                --enable-newlib-retargetable-locking
                #--disable-threads
            popd
            ;;
        libdragon)
            mkdir -p libdragon-build
            ;;
        make)
            if [ -z "$MAKE_SOURCE_DIR" ]; then
                mode_fetch make
                make_src=../"make-$MAKE_V"
            else
                make_src=$MAKE_SOURCE_DIR
            fi
            mkdir -p make-build
            pushd make-build
            $make_src/configure \
              --prefix="$INSTALL_PATH" \
                --disable-largefile \
                --disable-nls \
                --disable-rpath \
                --host="$N64_BUILD"
            popd
            ;;
        *) perror "Unknown configure target: $1"; return 1;;
    esac
}

mode_make () {
    local builddir
    local target
    local extra=
    if [ "$1" = rustc_codegen_gcc ]; then
        mode_configure $1
        local target buildpath=$(realpath "$BUILD_PATH")
        pushd "${RCGCC_SOURCE_DIR:-rustc_codegen_gcc}"
        ./y.sh build --release
        popd
    else
        case $1 in
            binutils | gcc | gccjit | newlib | make | libdragon) builddir=$1-build;;
            *) perror "Unknown make target: $1"; return 1;;
        esac
        case $1 in
            gccjit) target=all-gcc; extra=TARGET-gcc=jit;;
            libdragon) target=(libdragon tools)
                       builddir=${LIBDRAGON_SOURCE_DIR:-libdragon}
                       extra=(BUILD_DIR=$(realpath $1-build) N64_INST=$N64_INST N64_TARGET=mips64vr-n64-elf);;
            *) target=all;;
        esac
        if [ ! -e "$builddir/Makefile" ]; then
            mode_configure $1
        fi
        make -C "$builddir" -j "$JOBS" ${target[@]} ${extra[@]}
    fi
}

mode_install () {
    if [ "$1" = rustc_codegen_gcc ]; then
        mode_make $1
        local target rcgcc_src=${RCGCC_SOURCE_DIR:-rustc_codegen_gcc}
        for abi in ${RUSTC_GCC_ABIS[@]}; do
            target=mips64vr-n64-elf$abi
            install -vDt "$SCRIPT_DIR/.cargo" $target.json
        done
        exec_priv install -vDt "$N64_INST/lib/" "$rcgcc_src/target/release/librustc_codegen_gcc.so"
    else
        local install_targets
        local builddir=$1-build
        local extra=
        case $1 in
            binutils | gcc | make) install_targets=(install-strip install-strip-target);;
            gccjit)                install_targets=all-gcc; extra=TARGET-gcc=jit.install-common;;
            newlib)                install_targets=install;;
            libdragon)             install_targets=(install tools-install)
                                   builddir=${LIBDRAGON_SOURCE_DIR:-libdragon}
                                   extra=(BUILD_DIR=$(realpath $1-build) N64_INST=$N64_INST N64_TARGET=mips64vr-n64-elf);;
            *) perror "Unknown install target: $1"; return 1;;
        esac
        mode_make $1
        exec_priv make -C "$builddir" ${install_targets[@]} ${extra[@]}
    fi
}

check_noargs () {
    for arg in $@; do
        if [ -n "$arg" ]; then perror "Unknown argument: $arg"; return 1; fi
    done
}

mode_clean () {
    rm -rf "$BUILD_PATH/gcc-build" "$BUILD_PATH/libdragon-build"
}

for arg in $@; do
    case $arg in -h | --help) mode_help echo; exit 0;; esac
done
if [[ $# -eq 0 ]]; then mode_help perror; exit 1; fi

case $1 in
    fetch | update | configure | make | install)
        mkdir -p "$BUILD_PATH"
        cd "$BUILD_PATH"
        repeat_mode $@
    ;;
    clean) shift; [ "$@" = all] || check_noargs $@; mode_clean;;
    help) shift; check_noargs $@; mode_help echo;;
    *) perror "Unknown command $1"; mode_help perror; exit 1;;
esac
