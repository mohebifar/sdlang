// In-memory record storage keyed by id. It lives on globalThis so every route of one
// server process shares it; a restart empties it.

export type StoredRecord = { id: string; created_at: string };

type State = { records: Map<string, { record: StoredRecord; seq: number }>; seq: number };

const holder = globalThis as typeof globalThis & { __tasksStore?: State };

function state(): State {
  holder.__tasksStore ??= { records: new Map(), seq: 0 };
  return holder.__tasksStore;
}

/** Stores a new record and returns a copy of it. */
export function insert<T extends StoredRecord>(record: T): T {
  const s = state();
  s.seq += 1;
  s.records.set(record.id, { record: { ...record }, seq: s.seq });
  return { ...record };
}

/** Every record, newest first by created_at (later inserts first on a tie). */
export function all<T extends StoredRecord>(): T[] {
  return [...state().records.values()]
    .sort((a, b) =>
      a.record.created_at === b.record.created_at
        ? b.seq - a.seq
        : b.record.created_at.localeCompare(a.record.created_at),
    )
    .map((e) => ({ ...e.record }) as T);
}

/** The record with this id, or null. */
export function find<T extends StoredRecord>(id: string): T | null {
  const e = state().records.get(id);
  return e ? ({ ...e.record } as T) : null;
}

/** Replaces the stored record with the same id and returns a copy of it. */
export function save<T extends StoredRecord>(record: T): T {
  const s = state();
  const existing = s.records.get(record.id);
  if (existing) {
    existing.record = { ...record };
  } else {
    s.seq += 1;
    s.records.set(record.id, { record: { ...record }, seq: s.seq });
  }
  return { ...record };
}
