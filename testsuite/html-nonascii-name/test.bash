#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p 'naïve dir'
touch a 'café.txt' 'naïve dir/日本語.txt' $'lat\xe9in.txt'
${tree} -H . > ${actual}/stdout
${tree} -H https://example.com/base > ${actual}/baseurl-stdout


fin
