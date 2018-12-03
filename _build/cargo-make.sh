#!/bin/bash

if [ -z ${1+x} ];
then
 echo "Missing version as first argument";
 exit 1
fi

if [ -z ${2+x} ];
then
 echo "Missing target as second argument";
 exit 1
fi

curl https://github.com/sagiegurari/cargo-make/releases/download/${1}/cargo-make-v${1}-${2}.zip -sSfL -o /tmp/cargo-make.zip;
unzip /tmp/cargo-make.zip;
mv cargo-make-*/* $HOME/.cargo/bin;
echo "##vso[task.setvariable variable=PATH;]$PATH:$HOME/.cargo/bin"
