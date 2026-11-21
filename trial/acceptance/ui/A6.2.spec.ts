import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";
import { addTask } from "./add";

const server = freshServer();

test("A6.2 the new task page heading shows the same count", async ({ page }) => {
  await addTask(page, server.url(), "One");
  await addTask(page, server.url(), "Two");
  await page.goto(server.url() + routes.new);
  await expect(page.getByRole("heading", { level: 1 })).toContainText("(2)");
});
