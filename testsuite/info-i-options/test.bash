#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p A B
touch A/foo.txt
printf 'A/foo.txt\n\tA comment\n\tsecond line\nB\n\tdir B\n' > .info
${tree} --info -i > ${actual}/stdout
${tree} --info -i -p > ${actual}/metadata

fin
