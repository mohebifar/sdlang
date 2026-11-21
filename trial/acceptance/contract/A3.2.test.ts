import { expect, it } from "vitest";
import { action } from "./arm";

it("A3.2 set_task_done on an unknown id fails with not_found", async () => {
  await expect((await action("set_task_done"))({ id: "no-such-task", done: true })).rejects.toMatchObject({
    code: "not_found",
  });
});
