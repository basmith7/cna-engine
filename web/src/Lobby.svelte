<script lang="ts">
  // The online lobby (spec, *Web client*): the caller's games, those
  // awaiting their move first; public games waiting for a player; and a
  // form to make one, whose invite link shows once.
  import { sideName } from "./clock";
  import { type Account, type AdminAccount, type Created, type CreateSide, type MyGame, type OpenGame, accounts, createGame, freeSides, inviteLink, joinGame, leaveGame, myGames, openGames, setSuspended, signOut } from "./lobby";
  import type { Side } from "./types";

  let { account, onback }: { account: Account; onback: () => void } = $props();

  let mine: MyGame[] = $state([]);
  let open: OpenGame[] = $state([]);
  let error: string | null = $state(null);
  let busy = $state(false);
  let created: Created | null = $state(null);
  let copied = $state(false);

  let side: CreateSide = $state("either");
  let listed = $state(true);
  let openInformation = $state(false);

  /** The admin's account list; empty for anyone else. */
  let all: AdminAccount[] = $state([]);

  async function refresh() {
    try {
      [mine, open] = await Promise.all([myGames(), openGames()]);
      if (account.admin) all = await accounts();
      error = null;
    } catch (e) {
      error = (e as Error).message;
    }
  }
  refresh();

  async function act(f: () => Promise<void>) {
    busy = true;
    try {
      await f();
      await refresh();
    } catch (e) {
      error = (e as Error).message;
    } finally {
      busy = false;
    }
  }

  const create = () =>
    act(async () => {
      created = await createGame({ side, open: listed, open_information: openInformation });
      copied = false;
    });

  const join = (g: OpenGame, side: Side) => act(() => joinGame(g.id, side));
  const leave = (g: MyGame) => {
    if (confirm(`Leave game ${g.id}? It ends for both players.`)) act(() => leaveGame(g.id));
  };
  const suspend = (a: AdminAccount) => act(() => setSuspended(a.id, !a.suspended));

  const link = $derived(created ? inviteLink(location.origin, created) : "");

  async function copy() {
    await navigator.clipboard.writeText(link);
    copied = true;
  }

  async function out() {
    await signOut().catch(() => {});
    onback();
  }

  function statusLine(g: MyGame): string {
    if (g.status === "open") return "waiting for a player";
    if (g.status !== "active") return g.status;
    return g.my_move ? "your move" : `waiting for ${g.opponent ?? "your opponent"}`;
  }

  const creatorSide = (s: OpenGame["creator_side"]) => (s === "either" ? "either side" : sideName(s));
</script>

<div class="lobby" data-testid="lobby">
  <section class="card">
    <header>
      <h1>Play online</h1>
      <span class="who">{account.name} · <button type="button" class="link" onclick={out}>sign out</button> · <button type="button" class="link" onclick={onback}>back</button></span>
    </header>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <h2>My games</h2>
    {#if mine.length === 0}
      <p class="none">None yet.</p>
    {:else}
      <ul data-testid="my-games">
        {#each mine as g (g.id)}
          <li class:move={g.my_move}>
            {#if g.my_side}<a href={`/game/${g.id}`}>Game {g.id}</a>{:else}<span>Game {g.id}</span>{/if}
            <span>{g.my_side ? sideName(g.my_side) : "side at the join"}{g.opponent ? ` against ${g.opponent}` : ""}</span>
            <span class="state">{statusLine(g)}</span>
            {#if g.status === "open" || g.status === "active"}
              <button type="button" class="link" disabled={busy} onclick={() => leave(g)}>leave</button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    <h2>Open games</h2>
    {#if open.length === 0}
      <p class="none">Nobody is waiting for a player.</p>
    {:else}
      <ul data-testid="open-games">
        {#each open as g (g.id)}
          <li>
            <span>{g.creator ?? "someone"} as {creatorSide(g.creator_side)}</span>
            {#each freeSides(g) as side (side)}
              <button type="button" disabled={busy} onclick={() => join(g, side)}>Join as {sideName(side)}</button>
            {/each}
          </li>
        {/each}
      </ul>
    {/if}

    <h2>New game</h2>
    <form onsubmit={(e) => (e.preventDefault(), create())}>
      <p>
        <label>I play
          <select bind:value={side} aria-label="Side">
            <option value="either">either side</option>
            <option value="axis">Axis</option>
            <option value="cw">Commonwealth</option>
          </select>
        </label>
      </p>
      <p><label><input type="checkbox" bind:checked={listed} /> Listed in the open games</label></p>
      <p><label><input type="checkbox" bind:checked={openInformation} /> Open information (nothing hidden)</label></p>
      <p><button type="submit" class="primary" disabled={busy}>Create</button></p>
    </form>
    {#if created}
      <div class="invite" data-testid="invite">
        <p>Send this link to your opponent. It is shown only now.</p>
        <p><input type="text" readonly value={link} aria-label="Invite link" /> <button type="button" onclick={copy}>{copied ? "Copied" : "Copy"}</button></p>
      </div>
    {/if}

    <h2>Settings</h2>
    <p><label><input type="checkbox" checked={account.alerts} disabled /> Tell me by Discord message when it is my move</label></p>
    <p class="none">Turn alerts are not sent yet; this switch comes with them.</p>

    {#if account.admin}
      <h2>Accounts</h2>
      <ul data-testid="accounts">
        {#each all as a (a.id)}
          <li>
            <span>{a.name}{a.admin ? " (admin)" : ""}</span>
            <span class="state">{a.suspended ? "suspended" : "active"}</span>
            {#if a.id !== account.id}
              <button type="button" disabled={busy} onclick={() => suspend(a)}>{a.suspended ? "Restore" : "Suspend"}</button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .lobby {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    font: 15px/1.4 system-ui, sans-serif;
    color: #1b1b1b;
    overflow: auto;
  }
  .card {
    width: min(100%, 520px);
    min-width: 0;
    padding: 1.6rem 1.8rem;
    background: #f4f0e6;
    border: 1px solid #d8d1c0;
    border-radius: 8px;
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.6rem;
    margin-bottom: 0.6rem;
  }
  h1 {
    margin: 0;
    font-size: 24px;
  }
  h2 {
    margin: 1rem 0 0.4rem;
    font-size: 16px;
  }
  .who,
  .none,
  .state {
    color: #555;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
    padding: 0.3rem 0;
    border-bottom: 1px solid #e2dccd;
  }
  li.move .state {
    color: #1f2a33;
    font-weight: 600;
  }
  p {
    margin: 0 0 0.6rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.6rem;
  }
  button {
    font: inherit;
    padding: 0.35rem 0.8rem;
    background: #f2efe6;
    color: #1b1b1b;
    border: 1px solid #b9b2a2;
    border-radius: 4px;
    cursor: pointer;
  }
  button.primary {
    background: #1f2a33;
    color: #f2efe6;
    border-color: #0f161c;
  }
  button.link {
    padding: 0;
    background: none;
    border: none;
    color: #1f2a33;
    text-decoration: underline;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  a {
    color: #1f2a33;
  }
  select,
  input[type="text"] {
    font: inherit;
    padding: 0.25rem 0.4rem;
  }
  .invite input {
    flex: 1;
    min-width: 0;
  }
  .error {
    display: block;
    padding: 0.5rem 0.6rem;
    background: #fbe3e1;
    border: 1px solid #d9847d;
    border-radius: 4px;
    color: #7a1a1d;
    overflow-wrap: anywhere;
  }
</style>
