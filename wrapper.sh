#!/bin/bash

set -euo pipefail
IFS=$'\n\t'

if [ -z "${N64_INST-}" ]; then
    echo "N64_INST environment variable is not defined." > /dev/stderr
    echo "Please define N64_INST and point it to the requested installation directory before running cargo" > /dev/stderr
    exit 1
fi

export LD_LIBRARY_PATH=$N64_INST/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}

exec $@
