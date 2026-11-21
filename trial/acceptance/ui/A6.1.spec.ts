import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";
import { addTask } from "./add";

const server = freshServer();

test("A6.1 the list heading shows the open count, e.g. 'Tasks (2)'", async ({ page }) => {
  await addTask(page, server.url(), "One");
  await addTask(page, server.url(), "Two");
  await page.goto(server.url() + routes.index);
  await expect(page.getByRole("heading", { level: 1 })).toContainText("(2)");
});
