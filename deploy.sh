#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

mkdir dist
cp -r assets dist/
cp -r pkg dist/
cp index.html dist/
curl -L -o butler.zip https://broth.itch.ovh/butler/windows-amd64/LATEST/archive/default
unzip butler.zip
chmod +x butler.exe
./butler.exe -V
./butler.exe push dist boringcactus/dynamaze:web
