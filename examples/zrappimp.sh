#!/bin/sh

set -e
SCRIPT_DIR="$(dirname $(readlink -f "$0"))"

# Ersätt med dina uppgifter
export DINKASSA_USERNAME=''
export DINKASSA_PASSWORD=''
#export DINKASSA_INTEGRATOR_ID=''
#export DINKASSA_MACHINE_ID=''
#export DINKASSA_MACHINE_KEY=''
export BOKIO_API_TOKEN=''
export BOKIO_COMPANY_ID=''
export RUST_BACKTRACE=1

esbokio=esbokio
for d in "$SCRIPT_DIR" "$SCRIPT_DIR/../target/debug" "$SCRIPT_DIR/../target/release"; do
    if [ -x "$d/esbokio" ]; then
        esbokio="$d/esbokio"
        break
    fi
done

"$esbokio" "$@"
