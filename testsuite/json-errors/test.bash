#!/usr/bin/env bash
# -*- coding: utf-8 -*-

home=$(cd $(dirname $0) && pwd)
source ${home}/../initialize.bash

# Root can read 000-mode directories; the fixture is meaningless then.
if [[ $(id -u) -eq 0 ]]; then
    exit 0
fi

mkdir -p j1/sub j1/locked empty
touch j1/afile j1/zfile j1/sub/x plain
chmod 000 j1/locked

${tree} -J j1 > ${actual}/unreadable 2>&1
echo $? > ${actual}/unreadable-exit
${tree} -J --du j1 > ${actual}/unreadable-du 2>&1
echo $? > ${actual}/unreadable-du-exit
${tree} -J --prune j1 > ${actual}/unreadable-prune 2>&1
echo $? > ${actual}/unreadable-prune-exit
${tree} -J empty j1/sub > ${actual}/empty-first-root 2>&1
${tree} -J j1/sub empty > ${actual}/empty-last-root 2>&1
${tree} -J plain j1/sub > ${actual}/file-root 2>&1
${tree} -J --filelimit 1 j1 j1/sub > ${actual}/filelimit-root 2>&1

chmod 755 j1/locked

fin
