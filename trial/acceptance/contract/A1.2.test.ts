import { expect, it } from "vitest";
import { action } from "./arm";

it("A1.2 delete_task on an unknown id fails with not_found", async () => {
  await expect((await action("delete_task"))({ id: "no-such-task" })).rejects.toMatchObject({ code: "not_found" });
});
