import { expect, it } from "vitest";
import { action, markDone, tick } from "./arm";

it("B5 list_tasks orders open before done, newest first in each group", async () => {
  const create = await action("create_task");
  const ids: string[] = [];
  for (const title of ["one", "two", "three", "four"]) {
    ids.push((await create({ title })).id);
    await tick();
  }
  await markDone(ids[0]);
  await markDone(ids[2]);
  const listed = (await (await action("list_tasks"))({})).map((t: { id: string }) => t.id);
  expect(listed.filter((id: string) => ids.includes(id))).toEqual([ids[3], ids[1], ids[2], ids[0]]);
});
