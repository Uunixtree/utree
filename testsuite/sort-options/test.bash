#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p dir2 dir1
touch z1 z10 z2 dir1/a
printf 'xxxxxxxx' > big
printf 'x' > small
touch -t 202001010000 old-file
touch -t 202401010000 mid-file

${tree} -v > ${actual}/v
${tree} -r > ${actual}/r
${tree} -t > ${actual}/t
${tree} -c > ${actual}/c
${tree} --dirsfirst > ${actual}/dirsfirst
${tree} --filesfirst -r > ${actual}/filesfirst-r
${tree} --sort=size > ${actual}/sort-size
${tree} --sort=version -r > ${actual}/sort-version-r
${tree} -U --dirsfirst > ${actual}/U-dirsfirst

mkdir stamp
touch -t 202001010000 stamp/old-file stamp
${tree} --sort=ctime -D --timefmt '%Y' stamp > ${actual}/sort-ctime-D
${tree} -c --sort=name -D --timefmt '%Y' stamp > ${actual}/c-sort-name-D
${tree} -c -t -D --timefmt '%Y' stamp > ${actual}/c-t-D
${tree} -c --sort=mtime -D --timefmt '%Y' stamp > ${actual}/c-sort-mtime-D

fin
