#!/usr/bin/env bash
# The multiplayer e2e's server (playwright.multiplayer.config.ts): a fresh
# test-mode build of web/, the fake Discord and cna-server (test-auth)
# serving it, all native. Postgres is DATABASE_URL when set (CI's service),
# else a throwaway container of our own on CNA_PW_DB_PORT, removed on exit.
set -euo pipefail
cd "$(dirname "$0")/../.."
web=$PWD
port=${CNA_PW_PORT:-3260}
fake_port=${CNA_PW_FAKE_PORT:-3261}
db_port=${CNA_PW_DB_PORT:-5436}
container=cna-pw-db-$db_port
own_db=
pids=()

cleanup() {
  for p in "${pids[@]}"; do kill "$p" 2>/dev/null || true; done
  if [[ -n $own_db ]]; then docker rm -f "$container" >/dev/null 2>&1 || true; fi
}
trap cleanup EXIT
trap 'exit 0' TERM INT

if [[ -z ${DATABASE_URL:-} ]]; then
  own_db=1
  docker rm -f "$container" >/dev/null 2>&1 || true
  docker run -d --name "$container" -p "127.0.0.1:$db_port:5432" \
    -e POSTGRES_USER=cna -e POSTGRES_PASSWORD=cna -e POSTGRES_DB=cna postgres:18 >/dev/null
  until docker exec "$container" pg_isready -U cna -h 127.0.0.1 >/dev/null 2>&1; do sleep 0.5; done
  export DATABASE_URL=postgres://cna:cna@127.0.0.1:$db_port/cna
fi

npm run build -- --mode test --outDir dist-multiplayer --emptyOutDir
cargo build -p cna-server --features test-auth,fake --bins
target=$(cargo metadata --format-version 1 --no-deps | node -e 'process.stdout.write(JSON.parse(require("fs").readFileSync(0, "utf8")).target_directory)')
bin=$target/debug

# A bot token made per run: the fake refuses any other on its bot routes.
bot_token=$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')
FAKE_DISCORD_BIND=127.0.0.1:$fake_port FAKE_DISCORD_BOT_TOKEN=$bot_token "$bin/fake-discord" &
pids+=($!)
CNA_BIND=127.0.0.1:$port CNA_DEV=1 CNA_PUBLIC_URL=http://localhost:$port \
  CNA_WEB_DIST="$web/dist-multiplayer" \
  CNA_DISCORD_CLIENT_ID=dev CNA_DISCORD_CLIENT_SECRET=dev \
  CNA_DISCORD_AUTHORIZE=http://localhost:$fake_port/oauth2/authorize \
  CNA_DISCORD_API=http://localhost:$fake_port/api \
  CNA_DISCORD_BOT_TOKEN=$bot_token CNA_DISCORD_GUILD_ID=4242 \
  RUST_LOG=${RUST_LOG:-warn} "$bin/cna-server" &
pids+=($!)
# Either dying ends the run; Playwright's TERM ends both.
wait -n
