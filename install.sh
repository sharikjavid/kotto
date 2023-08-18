#!/usr/bin/env bash
deno \
  install \
  --allow-read="." \
  --allow-read="~/.config/trackway" \
  --allow-write="." \
  --allow-write="~/.config/trackway" \
  --allow-env="OPENAI_KEY" \
  --allow-net="api.openai.com" \
  --allow-net="trackway.ai" \
  --allow-run="trackwayc" \
  --name="trackway" \
  https://trackway.ai/cli.ts

trackway upgrade

trackway --help