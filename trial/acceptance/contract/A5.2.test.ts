import { expect, it } from "vitest";
import { action } from "./arm";

it("A5.2 list_tasks items carry done_at consistent with set_task_done", async () => {
  const create = await action("create_task");
  const set = await action("set_task_done");
  const done = (await create({ title: "done" })).id;
  const open = (await create({ title: "open" })).id;
  const marked = await set({ id: done, done: true });
  const items = await (await action("list_tasks"))({});
  const byId = (id: string) => items.find((t: { id: string }) => t.id === id);
  expect(byId(done).done_at).toBe(marked.done_at);
  expect(byId(open).done_at).toBeNull();
});
