#!/bin/bash

KEY="$1"

if [ -z "$KEY" ]; then
    echo "Usage: ./fetch-from-etcd.sh KEY" >&2
    exit 1
fi

while ! etcdctl get "$KEY" > "${KEY}.json"; do
    sleep 1
done
