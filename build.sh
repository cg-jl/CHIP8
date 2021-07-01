#!/bin/bash

build() {
  local pwd=$(pwd)
  pushd $1 >/dev/null && \
  cargo build --release && \
  cp ./target/release/$1 $pwd/bin && \
  popd > /dev/null
}

mkdir -p bin

build chip8-assembler
build chip8-interpreter
build chip8-decompiler
