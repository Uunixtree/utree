#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir d
touch 'd/foo\' d/foo d/bar
printf 'foo\\\n' > d/.gitignore
${tree} --gitignore d > ${actual}/gitignore
${tree} -P 'foo\' d > ${actual}/P-option
${tree} -I 'foo\' d > ${actual}/I-option
printf 'foo\\\n\tcomment\\\nbar\n\tpath C:\\\n' > d/.info
${tree} --info d > ${actual}/info

fin
