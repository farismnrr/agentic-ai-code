#!/bin/sh
set -eu

git config core.hooksPath .githooks

echo "Git hooks enabled: core.hooksPath=.githooks"
