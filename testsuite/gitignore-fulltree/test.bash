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

mkdir -p repo/.git repo/A other/.git
echo secret > repo/.gitignore
echo local > repo/A/.gitignore
touch repo/A/secret repo/A/local repo/A/kept other/secret other/local other/kept
${tree} --gitignore repo/A other > ${actual}/roots-plain
${tree} --gitignore --prune repo/A other > ${actual}/roots-prune
${tree} --gitignore --du repo/A other > ${actual}/roots-du

fin
