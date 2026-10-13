// One English line per engine Event for the log panel (spec, *Log*).
// Exhaustive: a new variant in `types.ts` fails `tsc` at the `never`.
// Our own words, never the rulebook's.
import { PHASE_NAMES, SEGMENT_NAMES, sideName } from "./clock";
import type { BarrageResult, Event, Location, PieceId, RetreatReason, Weather } from "./types";

/** How a piece is named in a line; the app passes the view's labels. */
export type Namer = (id: PieceId) => string;

const plain: Namer = (id) => id;

function list(ids: PieceId[], name: Namer): string {
  return ids.length === 0 ? "nothing" : ids.map(name).join(", ");
}

function where(loc: Location): string {
  if (loc === "eliminated") return "eliminated";
  if ("hex" in loc) return loc.hex;
  if ("box" in loc) return `the ${loc.box} box`;
  if ("transit" in loc) return `in transit ${loc.transit.from} → ${loc.transit.to}`;
  return "to be placed";
}

function weather(w: Weather): string {
  if (typeof w === "string") return w;
  if ("sandstorm" in w) return `sandstorm on ${w.sandstorm.sheets.join(", ")}`;
  return `rainstorm on ${w.rainstorm.sheets.join(", ")}`;
}

function barrageResult(r: BarrageResult): string {
  if (typeof r === "string") return r === "no-effect" ? "no effect" : r;
  return `lose ${r.lose}`;
}

const REASON: Record<RetreatReason, string> = {
  "assault-result": "the assault result",
  withheld: "withheld forces",
  "must-attack": "must attack or retreat",
};

const STEP = { "gun-positions": "gun positions", barrage: "barrage", "assign-forces": "force assignment" } as const;

export function formatEvent(e: Event, name: Namer = plain): string {
  if (e === "set-up-complete") return "Set-up complete; play begins.";
  if ("phase-started" in e) {
    const c = e["phase-started"].clock;
    return `Game-Turn ${c.game_turn} OpStage ${c.stage}: ${PHASE_NAMES[c.phase]}${c.half ? ` (Player ${c.half})` : ""}${c.segment ? `, ${SEGMENT_NAMES[c.segment]}` : ""} begins.`;
  }
  if ("phase-skipped" in e) {
    const c = e["phase-skipped"].clock;
    return `${c.segment ?? c.phase}: nothing to do, skipped.`;
  }
  if ("die-rolled" in e) return `Die for ${e["die-rolled"].purpose}: ${e["die-rolled"].value}.`;
  if ("initiative-determined" in e) {
    const d = e["initiative-determined"];
    return `${sideName(d.holder)} holds the initiative${d.rolled ? " (by die roll)" : ""}.`;
  }
  if ("initiative-declared" in e) return `${sideName(e["initiative-declared"].player_a)} declares itself Player A.`;
  if ("weather-set" in e) return `Weather: ${weather(e["weather-set"].weather)}.`;
  if ("placed" in e) return `${name(e.placed.piece)} placed at ${e.placed.hex}.`;
  if ("arrived" in e) return `${list(e.arrived.pieces, name)} arrive at ${where(e.arrived.at)}.`;
  if ("moved-off-map" in e) return `${name(e["moved-off-map"].piece)} moves off-map to ${where(e["moved-off-map"].location)}.`;
  if ("moved" in e) return `${list(e.moved.pieces, name)}: ${e.moved.from} → ${e.moved.to} for ${e.moved.cost} CP.`;
  if ("disorganised" in e) {
    const d = e.disorganised;
    return `${name(d.piece)} takes ${d.dp} DP, now cohesion level ${d.level}.`;
  }
  if ("move-interrupted" in e) {
    const d = e["move-interrupted"];
    return `${list(d.pieces, name)} stop at ${d.at} before ${d.before}: ZOC of ${d.declare.join(", ")} to be declared.`;
  }
  if ("zoc-declared" in e) return `${e["zoc-declared"].hex} ${e["zoc-declared"].exerts ? "exerts" : "does not exert"} a ZOC.`;
  if ("reaction-offered" in e) return `Reaction offered to ${list(e["reaction-offered"].pieces, name)} against ${e["reaction-offered"].at}.`;
  if ("contact-broken" in e) return `${name(e["contact-broken"].piece)} breaks contact for ${e["contact-broken"].cost} CP.`;
  if ("reserve-designated" in e) return `${list(e["reserve-designated"].pieces, name)} designated reserve.`;
  if ("reserve-released" in e) return `${list(e["reserve-released"].pieces, name)} released from Reserve ${e["reserve-released"].from}.`;
  if ("reserve-flipped" in e) return `${list(e["reserve-flipped"].pieces, name)} flip from Reserve I to Reserve II.`;
  if ("sealed" in e) return `${sideName(e.sealed.side)} has sealed its ${STEP[e.sealed.step]}.`;
  if ("gun-positions-set" in e) {
    const d = e["gun-positions-set"];
    return `${sideName(d.side)} guns forward: ${list(d.forward, name)}.`;
  }
  if ("barrage-plotted" in e) {
    const d = e["barrage-plotted"];
    const fires = d.fires.map((f) => `${name(f.piece)} ${f.points} at ${name(f.target)}`).join("; ");
    return `${sideName(d.side)} barrage plotted: ${fires || "none"}.`;
  }
  if ("barrage" in e) {
    const d = e.barrage;
    return `${sideName(d.side)} barrage on ${name(d.target)} by ${list(d.firers, name)}: ${d.points} points, shift ${d.shift > 0 ? `+${d.shift}` : d.shift}, reading ${d.reading}: ${barrageResult(d.result)}.`;
  }
  if ("retreated-before-assault" in e) return `${list(e["retreated-before-assault"].pieces, name)} retreat before assault to ${e["retreated-before-assault"].to}.`;
  if ("anti-armour" in e) {
    const d = e["anti-armour"];
    return `${sideName(d.side)} anti-armour fire at ${d.target}: ${d.points} points, shift ${d.shift > 0 ? `+${d.shift}` : d.shift}, reading ${d.reading}: ${d.damage} point${d.damage === 1 ? "" : "s"} of damage.`;
  }
  if ("assault" in e) {
    const d = e.assault;
    const sign = (n: number) => (n > 0 ? `+${n}` : `${n}`);
    return `${d.probe ? "Probe" : "Assault"} on ${d.target} by ${list(d.attackers, name)}: ${d.attacker_raw} v ${d.defender_raw} raw, differential ${sign(d.basic)}, ${sign(d.shifts)} shifts, column ${d.column}; attacker rolls ${d.attacker.reading} and loses ${d.attacker.pct}% (${d.attacker.toe_lost} TOE), defender rolls ${d.defender.reading} and loses ${d.defender.pct}% (${d.defender.toe_lost} TOE)${d.defender.retreat ? `, retreats ${d.defender.retreat}` : ""}${d.defender.engaged ? ", engaged" : ""}${d.defender.captured ? ", captured" : ""}.`;
  }
  if ("prisoners" in e) return `${sideName(e.prisoners.side)} takes ${e.prisoners.points} prisoner points.`;
  if ("surrendered" in e) return `${list(e.surrendered.pieces, name)} surrender.`;
  if ("engaged" in e) return `${list(e.engaged.pieces, name)} are engaged.`;
  if ("retreat-ordered" in e) {
    const d = e["retreat-ordered"];
    return `${list(d.pieces, name)} must retreat ${d.hexes} hex${d.hexes === 1 ? "" : "es"} (${REASON[d.reason]}).`;
  }
  if ("retreat-bought-out" in e) {
    const d = e["retreat-bought-out"];
    return `${list(d.pieces, name)} stand ${d.short} hex${d.short === 1 ? "" : "es"} short of the ordered retreat and pay for it.`;
  }
  if ("losses" in e) return `${name(e.losses.piece)} loses ${e.losses.points} points, TOE now ${e.losses.toe}.`;
  if ("eliminated" in e) return `${name(e.eliminated.piece)} eliminated.`;
  if ("forces-assigned" in e) {
    const d = e["forces-assigned"];
    const parts = d.assignments.map((a) => {
      const aa = (a.anti_armour ?? []).map((x) => `${x.points} anti-armour${x.target ? ` at ${x.target}` : ""}`);
      const ca = (a.close_assault ?? []).map((x) => `${x.points} assault${x.target ? ` at ${x.target}` : ""}`);
      return `${name(a.piece)}: ${[...aa, ...ca].join(", ") || "nothing"}`;
    });
    return `${sideName(d.side)} forces assigned: ${parts.join("; ") || "none"}.`;
  }
  if ("combat-cp" in e) return `${name(e["combat-cp"].piece)} pays ${e["combat-cp"].cost} CP for combat.`;
  if ("combat-cp-refunded" in e) {
    const d = e["combat-cp-refunded"];
    return `${name(d.piece)} refunded ${d.cp} CP and ${d.dp} DP.`;
  }
  if ("captured" in e) return `${name(e.captured.piece)} captured at ${e.captured.hex}.`;
  if ("game-over" in e) {
    const r = e["game-over"].report;
    return `Game over. Axis: ${r.axis ?? "no level"}; Commonwealth: ${r.cw ?? "no level"}.`;
  }
  return unreachable(e);
}

function unreachable(e: never): never {
  throw new Error(`unknown event ${JSON.stringify(e)}`);
}
