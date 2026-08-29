#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p Example/dir1 Example/dir2 Example/dir3 Example/dir4 Example/dir5
echo '*' > Example/dir3/.gitignore
touch Example/dir1/visible.txt Example/dir2/visible.txt Example/dir4/visible.txt Example/dir5/visible.txt Example/dir3/ignored.txt

${tree} --gitignore Example > ${actual}/plain
${tree} --gitignore --du Example > ${actual}/du
${tree} --gitignore --prune Example > ${actual}/prune

fin
