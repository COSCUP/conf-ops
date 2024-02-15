#/bin/bash

cleanup() {
  kill -TERM $pid1
  exit 0
}

trap cleanup INT

cargo build
(cd client && pnpm run optimize)

cargo watch -i client/ -x run &
pid1=$!

sleep 1

(cd client && pnpm run dev)
