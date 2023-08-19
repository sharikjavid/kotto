#!/usr/bin/env bash
deno \
  install \
  -A \
  --name="trackway" \
  https://trackway.ai/cli.ts

export PATH="$HOME/.deno/bin:$PATH"

trackway upgrade

trackway --help