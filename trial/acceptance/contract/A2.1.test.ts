import { expect, it } from "vitest";
import { action, markDone } from "./arm";

it("A2.1 list_tasks({ status }) returns only tasks with that status", async () => {
  const create = await action("create_task");
  const open = (await create({ title: "open one" })).id;
  const done = (await create({ title: "done one" })).id;
  await markDone(done);
  const list = await action("list_tasks");
  const ids = async (status: string) => (await list({ status })).map((t: { id: string }) => t.id);
  expect(await ids("done")).toEqual([done]);
  expect(await ids("open")).toEqual([open]);
});
