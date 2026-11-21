import { expect, test } from "@playwright/test";
import { freshServer, routes, today } from "./server";
import { addTask, row } from "./add";

const server = freshServer();

test("B1 an added task is listed with its title and today's date as YYYY-MM-DD", async ({ page }) => {
  await addTask(page, server.url(), "Buy milk");
  await page.goto(server.url() + routes.index);
  await expect(row(page, "Buy milk")).toContainText(today());
});
