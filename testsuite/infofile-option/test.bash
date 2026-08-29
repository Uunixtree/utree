#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

mkdir -p binx bootx usrx
touch bootx/{a,b,c}

${tree} --infofile ${home}/_info > ${actual}/stdout



printf '%s/bootx/a\n\tabsolute pattern comment\n' "${fixture}" > abs_info
${tree} --infofile abs_info ${fixture} > ${actual}/absolute

fin
