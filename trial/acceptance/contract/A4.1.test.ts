import { expect, it } from "vitest";
import { action } from "./arm";

it("A4.1 create_task trims the title and collapses inner spaces, and stores it so", async () => {
  const created = await (await action("create_task"))({ title: "  Buy   milk " });
  expect(created.title).toBe("Buy milk");
  const listed = (await (await action("list_tasks"))({})).find((t: { id: string }) => t.id === created.id);
  expect(listed?.title).toBe("Buy milk");
});
