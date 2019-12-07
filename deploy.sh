#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

cargo build --release
mkdir dist
cp -r assets dist/
if [[ "$TRAVIS_OS_NAME" == "windows" ]]; then
  SUFFIX=.exe
else
  SUFFIX=
fi
cp target/release/dynamaze$SUFFIX dist/
if [[ "$TRAVIS_OS_NAME" == "osx" ]]; then
  BUTLER_DIST=darwin
else
  BUTLER_DIST=$TRAVIS_OS_NAME
fi
curl -L -o butler.zip https://broth.itch.ovh/butler/$BUTLER_DIST-amd64/LATEST/archive/default
unzip butler.zip
chmod +x butler$SUFFIX
./butler$SUFFIX -V
./butler$SUFFIX push dist boringcactus/dynamaze:$TRAVIS_OS_NAME
