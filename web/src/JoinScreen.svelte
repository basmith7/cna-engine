<script lang="ts">
  // An invite link (`/join/{id}?invite=…`, spec, *Web client*): sign in
  // first if need be, then take a seat. The server knows which seats are
  // free; a taken one is refused with its message.
  import { sideName } from "./clock";
  import { type Account, LobbyError, SIGN_IN_URL, joinGame } from "./lobby";
  import { RETURN_KEY } from "./route";
  import type { Side } from "./types";

  let { account, id, invite }: { account: Account | null; id: string; invite: string | null } = $props();

  let error: string | null = $state(null);
  let busy = $state(false);

  function signIn() {
    try {
      sessionStorage.setItem(RETURN_KEY, location.pathname + location.search);
    } catch {
      // Without storage the player comes back to the start screen and follows the link again.
    }
    location.assign(SIGN_IN_URL);
  }

  async function join(side: Side) {
    busy = true;
    try {
      await joinGame(Number(id), side, invite ?? undefined);
      location.assign(`/game/${id}`);
    } catch (e) {
      const message = (e as Error).message;
      error = e instanceof LobbyError && message === "that seat is taken" ? `The ${sideName(side)} seat is taken: try the other one.` : message;
      busy = false;
    }
  }
</script>

<div class="join" data-testid="join">
  <section class="card">
    <h1>Join game {id}</h1>
    {#if !account}
      <p>Sign in to take a seat in this game.</p>
      <p><button type="button" class="primary" onclick={signIn}>Sign in with Discord</button></p>
    {:else}
      <p>Signed in as {account.name}. Take the seat your opponent left free.</p>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <p>
        {#each ["axis", "cw"] as const as side (side)}
          <button type="button" class="primary" disabled={busy} onclick={() => join(side)}>Join as {sideName(side)}</button>
        {/each}
      </p>
      <p><a href={`/game/${id}`}>Already in it? Open the game</a> · <a href="/">Start screen</a></p>
    {/if}
  </section>
</div>

<style>
  .join {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    font: 15px/1.4 system-ui, sans-serif;
    color: #1b1b1b;
  }
  .card {
    width: min(100%, 460px);
    padding: 1.6rem 1.8rem;
    background: #f4f0e6;
    border: 1px solid #d8d1c0;
    border-radius: 8px;
  }
  h1 {
    margin: 0 0 0.6rem;
    font-size: 24px;
  }
  p {
    margin: 0 0 0.6rem;
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
  }
  button {
    font: inherit;
    padding: 0.35rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    background: #1f2a33;
    color: #f2efe6;
    border: 1px solid #0f161c;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  a {
    color: #1f2a33;
  }
  .error {
    display: block;
    padding: 0.5rem 0.6rem;
    background: #fbe3e1;
    border: 1px solid #d9847d;
    border-radius: 4px;
    color: #7a1a1d;
  }
</style>
