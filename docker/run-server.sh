#!/bin/bash

NUM_SHARDS="$1"

if [ -z "$NUM_SHARDS" ]; then
    echo "USAGE: ./run-server.sh NUM_SHARDS" >&2
    exit 1
fi

./fetch-from-etcd.sh "genesis"
./fetch-from-etcd.sh "server_$SERVER_ID"

for shard in $(seq 0 $(expr "${NUM_SHARDS}" - 1)); do
    ./server run \
        --storage "shard_${shard}.db" \
        --server "server_${SERVER_ID}.json" \
        --shard "$shard" \
        --genesis genesis.json &
done

sleep 15
