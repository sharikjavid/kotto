#!/usr/bin/env bash
deno \
  install \
  -A \
  --name="kotto" \
  https://kotto.land/cli.ts

export PATH="$HOME/.deno/bin:$PATH"

kotto upgrade

kotto --help