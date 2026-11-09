#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p data/a/b/c data/a/empty data/a/.hid data/x/y
head -c 102400 /dev/zero > data/a/b/c/deep.txt
head -c 10240 /dev/zero > data/a/b/mid.txt
head -c 51200 /dev/zero > data/a/top.txt
head -c 7168 /dev/zero > data/a/.hid/h.txt
head -c 33792 /dev/zero > data/x/y/z.txt
head -c 1024 /dev/zero > data/plain.txt

${tree} --du data > ${actual}/du
${tree} --du -L 1 data > ${actual}/du-L1
${tree} --du -L 2 data > ${actual}/du-L2
${tree} --du -a -L 1 data > ${actual}/du-a-L1
${tree} --du --prune data > ${actual}/du-prune
${tree} --du --prune -L 1 data > ${actual}/du-prune-L1
${tree} --du --prune -L 2 data > ${actual}/du-prune-L2
${tree} --du -J -L 2 data > ${actual}/du-J-L2
${tree} --prune -L 2 data > ${actual}/prune-L2

fin
