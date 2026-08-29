#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p d
touch d/aa d/bb

${tree} -P '[' d > ${actual}/P-unclosed-class 2>&1
${tree} -I '[' d > ${actual}/I-unclosed-class 2>&1
${tree} -P '|x' d > ${actual}/P-leading-bar 2>&1
${tree} -P 'x|' d > ${actual}/P-trailing-bar 2>&1
${tree} -P 'a*|b*' d > ${actual}/P-valid-alternation 2>&1

fin
