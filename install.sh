#!/usr/bin/env bash
deno \
  install \
  --allow-read="." \
  --allow-write="." \
  --allow-env="OPENAI_KEY" \
  --allow-net="api.openai.com" \
  --allow-net="damien.sh" \
  --allow-run="trackwayc" \
  --name="trackway" \
  https://damien.sh/trackway/cli.ts