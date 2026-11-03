#!/usr/bin/env bash
# The release image end to end, locally: docker-compose.prod.yml with a
# throwaway data dir and the fake Discord, then /healthz, a sign-in as
# alice by curl, a game created, and everything torn down.
#   deploy/smoke.sh [image]     (default cna:dev, built per the Dockerfile's header)
#
# The fake Discord runs natively (it is not in the release image), so the
# cna container shares the host network to reach it; deploy/smoke.override.yml
# says how.
set -euo pipefail
cd "$(dirname "$0")/.."
image=${1:-cna:dev}
port=3242 fake_port=3243 db_port=5437
data=$(mktemp -d /tmp/cna-smoke.XXXXXX)
jar=$data/cookies
project=cna-smoke
fake_pid=

compose() {
  CNA_IMAGE=$image CNA_DATA=$data CNA_SMOKE_PORT=$port CNA_SMOKE_DB_PORT=$db_port \
    POSTGRES_PASSWORD=smoke CNA_PUBLIC_URL=http://localhost:$port \
    CNA_DISCORD_CLIENT_ID=dev CNA_DISCORD_CLIENT_SECRET=dev \
    CNA_DISCORD_AUTHORIZE=http://localhost:$fake_port/oauth2/authorize \
    CNA_DISCORD_API=http://localhost:$fake_port/api \
    CNA_ADMINS=1001 RUST_LOG=warn \
    docker compose -p "$project" -f docker-compose.prod.yml -f deploy/smoke.override.yml "$@"
}
cleanup() {
  [[ -n $fake_pid ]] && kill "$fake_pid" 2>/dev/null || true
  compose down -v --remove-orphans >/dev/null 2>&1 || true
  # Postgres wrote the data dir as its own user.
  docker run --rm -v "$data:/d" debian:trixie-slim sh -c 'rm -rf /d/pg' >/dev/null 2>&1 || true
  rm -rf "$data"
}
trap cleanup EXIT

cargo build -q -p cna-server --features fake --bin fake-discord
target=$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
FAKE_DISCORD_BIND=127.0.0.1:$fake_port "$target/debug/fake-discord" &
fake_pid=$!

compose up -d --wait
base=http://localhost:$port
# Each check is assigned before it is printed, so a failing curl stops the run.
health=$(curl -fsS "$base/healthz"); echo "healthz: $health"
title=$(curl -fsS "$base/" | grep -o '<title>[^<]*</title>'); echo "index: $title"

# Sign in: our /auth/discord → the fake's consent page → alice's link → our callback.
consent=$(curl -fsS -c "$jar" -b "$jar" -o /dev/null -w '%{redirect_url}' "$base/auth/discord")
alice=$(curl -fsS "$consent" | grep -o 'href="[^"]*">alice<' | sed 's/^href="//; s/">alice<$//; s/&amp;/\&/g')
curl -fsS -c "$jar" -b "$jar" -o /dev/null "$alice"
me=$(curl -fsS -b "$jar" "$base/api/me"); echo "me: $me"

game=$(curl -fsS -b "$jar" -H 'Content-Type: application/json' -H "Origin: $base" \
  -d '{"side":"axis"}' "$base/api/games"); echo "created: $game"
mine=$(curl -fsS -b "$jar" "$base/api/games?mine"); echo "my games: $mine"
health=$(docker inspect -f '{{.State.Health.Status}}' "$(compose ps -q cna)")
echo "container: $health"
[[ $health == healthy ]]
echo "smoke: ok"
