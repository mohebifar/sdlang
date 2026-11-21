import { all, find, insert, save } from "../../../src/lib/tasks/store";

beforeEach(() => {
  delete (globalThis as { __tasksStore?: unknown }).__tasksStore;
});

it("[case:newest_first] all returns records newest first by created_at", () => {
  insert({ id: "a", created_at: "2026-01-01T10:00:00.000Z" });
  insert({ id: "c", created_at: "2026-01-03T10:00:00.000Z" });
  insert({ id: "b", created_at: "2026-01-02T10:00:00.000Z" });
  expect(all().map((r) => r.id)).toEqual(["c", "b", "a"]);
  save({ id: "a", created_at: "2026-01-01T10:00:00.000Z" });
  expect(all().map((r) => r.id)).toEqual(["c", "b", "a"]);
});

it("[case:unknown_id] find returns null for an unknown id", () => {
  insert({ id: "a", created_at: "2026-01-01T10:00:00.000Z" });
  expect(find("missing")).toBeNull();
  expect(find("a")).toEqual({ id: "a", created_at: "2026-01-01T10:00:00.000Z" });
});
