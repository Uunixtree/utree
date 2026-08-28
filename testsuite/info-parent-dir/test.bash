#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p A/sub
touch A/foo.txt A/sub/bar
printf 'A/foo.txt\n\tA comment\nA/sub/bar\n\tbar comment\nfoo.txt\n\troot-relative\n' > .info
${tree} --info A > ${actual}/plain
${tree} --info ./A > ${actual}/dot-slash
${tree} --info A/ > ${actual}/trailing-slash
${tree} --info "${fixture}/A" > ${actual}/absolute
${tree} --info ./A/sub > ${actual}/nested
${tree} --info --prune ./A > ${actual}/prune
${tree} --info -f ./A > ${actual}/full-path

fin
