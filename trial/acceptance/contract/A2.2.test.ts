import { expect, it } from "vitest";
import { action, markDone } from "./arm";

it("A2.2 list_tasks({}) returns all tasks", async () => {
  const create = await action("create_task");
  const open = (await create({ title: "open one" })).id;
  const done = (await create({ title: "done one" })).id;
  await markDone(done);
  const ids = (await (await action("list_tasks"))({})).map((t: { id: string }) => t.id);
  expect([...ids].sort()).toEqual([open, done].sort());
});
