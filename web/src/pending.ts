// The engine's interrupts, one by one (spec, *Interrupts*): what each
// `Pending` variant asks, in words, and the exhaustive switch the
// panels hang off. A new variant in `types.ts` fails `tsc` at the
// `never`, so the panel for it cannot be forgotten.
import { sideName } from "./clock";
import type { Pending, SealedStep } from "./types";

export type PendingKind = "set-up" | "declare-zoc" | "reaction" | "sealed" | "retreat-before-assault" | "forced-retreat";

export function pendingKind(p: Pending): PendingKind {
  if ("set-up" in p) return "set-up";
  if ("declare-zoc" in p) return "declare-zoc";
  if ("reaction" in p) return "reaction";
  if ("sealed" in p) return "sealed";
  if ("retreat-before-assault" in p) return "retreat-before-assault";
  if ("forced-retreat" in p) return "forced-retreat";
  return unreachable(p);
}

const STEP: Record<SealedStep, string> = { "gun-positions": "gun positions", barrage: "barrage", "assign-forces": "force assignment" };

/** The panel's heading for a pending. */
export function pendingTitle(p: Pending): string {
  if ("set-up" in p) return `Set-up · ${sideName(p["set-up"].side)}`;
  if ("declare-zoc" in p) return `ZOC declaration · ${sideName(p["declare-zoc"].side)}`;
  if ("reaction" in p) return `Reaction · ${sideName(p.reaction.side)}`;
  if ("sealed" in p) return `Sealed ${STEP[p.sealed.step]} · ${sideName(p.sealed.waiting[0] ?? null)}`;
  if ("retreat-before-assault" in p) return `Retreat before assault · ${sideName(p["retreat-before-assault"].side)}`;
  if ("forced-retreat" in p) return `Forced retreat · ${sideName(p["forced-retreat"].side)}`;
  return unreachable(p);
}

/** The ZOC question, in words. */
export function zocQuestion(p: { hexes: string[]; at_start: boolean }): string {
  const n = p.hexes.length;
  const these = n === 1 ? "this hex of yours exerts" : `which of these ${n} hexes of yours exert`;
  return p.at_start
    ? `The movement segment opens with enemy units next to you: say whether ${these} a zone of control (10.16).`
    : `An enemy stack is about to move next to you: say whether ${these} a zone of control (10.16).`;
}

function unreachable(p: never): never {
  throw new Error(`unknown pending ${JSON.stringify(p)}`);
}
