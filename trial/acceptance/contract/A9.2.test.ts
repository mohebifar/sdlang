import { expect, it } from "vitest";
import { action } from "./arm";

it("A9.2 create_task called directly with an 81-character title still succeeds", async () => {
  const title = "y".repeat(81);
  const created = await (await action("create_task"))({ title });
  expect(created.title).toBe(title);
});
