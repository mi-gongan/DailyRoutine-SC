#!/bin/sh

./build.sh

if [ $? -ne 0 ]; then
  echo ">> Error building contract"
  exit 1
fi

echo ">> Deploying contract"

# https://docs.near.org/tools/near-cli#near-dev-deploy
near dev-deploy --wasmFile ./target/wasm32-unknown-unknown/release/daily_routine.wasm --initFunction new --initArgs '{"owner_id": "inbum.testnet"}'
# near dev-deploy --wasmFile ./target/wasm32-unknown-unknown/release/daily_routine.wasm 
