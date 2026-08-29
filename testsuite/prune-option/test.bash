#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p b/.d .e/g h/j
touch a b/c .e/f h/{i,j/.l}
${tree} --prune > ${actual}/stdout


mkdir -p outer/hello outer/vacant
touch outer/README outer/hello/file
${tree} -L 2 --prune outer > ${actual}/level2
${tree} -L 1 --prune outer > ${actual}/level1

fin
