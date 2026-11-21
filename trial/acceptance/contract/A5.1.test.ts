import { expect, it } from "vitest";
import { action } from "./arm";

it("A5.1 set_task_done returns done_at set when done, null when open", async () => {
  const { id } = await (await action("create_task"))({ title: "timed" });
  const set = await action("set_task_done");
  const done = await set({ id, done: true });
  expect(typeof done.done_at).toBe("string");
  expect(Number.isNaN(Date.parse(done.done_at))).toBe(false);
  const open = await set({ id, done: false });
  expect(open.done_at).toBeNull();
});
