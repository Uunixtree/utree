#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p sub
printf 'hello world' > sub/file.txt
printf 'x' > tiny
head -c 2048 /dev/zero > big.bin
chmod 754 big.bin
chmod 600 tiny
ln -s sub linked
ln -s tiny to-file
touch -t 202001151200 old-file
mkfifo pipe 2>/dev/null || touch pipe
# Pin every timestamp so both passes see identical dates.
touch -t 202001151200 -h . sub sub/file.txt tiny big.bin linked to-file old-file pipe

${tree} -p > ${actual}/p
${tree} -s > ${actual}/s
${tree} -s -h > ${actual}/sh
${tree} -s --si > ${actual}/si
${tree} -u -g > ${actual}/ug
${tree} -D --timefmt=%Y-%m-%d > ${actual}/timefmt
${tree} -F > ${actual}/F
${tree} -Q > ${actual}/Q
${tree} -p -u -g -s -D --timefmt=%s > ${actual}/combined
${tree} --inodes --device | sed -E 's/\[ *[0-9]+/[INODE/' > ${actual}/inodes-device
${tree} --du > ${actual}/du
${tree} --du -h > ${actual}/du-h
${tree} --du --sort=size > ${actual}/du-sort-size
${tree} -J -D --timefmt '%Y"%m\%d' sub > ${actual}/J-timefmt-escape
${tree} -X -D --timefmt '%Y&%m<%d>"' sub > ${actual}/X-timefmt-escape

fin
