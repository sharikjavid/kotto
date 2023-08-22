#!/usr/bin/env bash
deno install -f -A --name="kotto" https://kotto.land/cli.ts
export PATH="$HOME/.deno/bin:$PATH"
kotto upgrade
kotto --help