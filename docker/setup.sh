#!/bin/bash -x

NUM_VALIDATORS="$1"
NUM_SHARDS="$2"

if [ -z "$NUM_VALIDATORS" ] || [ -z "$NUM_SHARDS" ]; then
    echo "USAGE: ./setup.sh NUM_VALIDATORS NUM_SHARDS" >&2
    exit 1
fi

# Start etcd server
etcd \
    --listen-client-urls 'http://0.0.0.0:2379' \
    --advertise-client-urls 'http://zefchain-setup-1:2379' &

# Clean up data files
rm -rf config/*

# Creare validator configuration directories and generate the command line options
validator_options() {
    for server in $(seq 1 ${NUM_VALIDATORS}); do
        echo "server_${server}.json:tcp:zefchain-server_${server}-1:9100:${NUM_SHARDS}"
    done
}

# Create configuration files for ${NUM_VALIDATORS} validators with ${NUM_SHARDS} shards each.
# * Private server states are stored in `server*.json`.
# * `committee.json` is the public description of the FastPay committee.
VALIDATORS=($(validator_options))
./server generate-all --validators ${VALIDATORS[@]} --committee committee.json

# Create configuration files for 1000 user chains.
# * Private chain states are stored in one local wallet `wallet.json`.
# * `genesis.json` will contain the initial balances of chains as well as the initial committee.
./client \
    --wallet wallet.json \
    --genesis genesis.json \
    create_genesis_config 1000 \
    --initial-funding 100 \
    --committee committee.json

etcdctl set "genesis" "$(cat genesis.json)"
etcdctl set "wallet" "$(cat wallet.json)"

for server in $(seq 1 ${NUM_VALIDATORS}); do
    etcdctl set "server_${server}" "$(cat "server_${server}.json")"
done

# Keep etcd server running on the foreground
fg 1
