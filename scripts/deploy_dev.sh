#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

make -C virdant.org build

rsync -a --delete virdant.org/build/ digitalocean:/root/projects/virdant_dev/docs/build/html/
