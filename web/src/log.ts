// The log panel's entries (spec, *Log*): event lines grouped under clock
// headers, and the engine's refusals in red, which are not part of the
// game log. Pure functions over an array; the app holds the array.

export type LogEntry = { kind: "header"; text: string } | { kind: "event"; text: string } | { kind: "illegal"; text: string };

/** Appends event lines under `header`, which is written only when it differs from the last header. */
export function withEvents(log: readonly LogEntry[], header: string, lines: string[]): LogEntry[] {
  if (lines.length === 0) return [...log];
  const last = [...log].reverse().find((e) => e.kind === "header");
  const out: LogEntry[] = [...log];
  if (last?.text !== header) out.push({ kind: "header", text: header });
  for (const text of lines) out.push({ kind: "event", text });
  return out;
}

export function withIllegal(log: readonly LogEntry[], text: string): LogEntry[] {
  return [...log, { kind: "illegal", text }];
}
