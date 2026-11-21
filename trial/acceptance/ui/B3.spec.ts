import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";
import { addTask, row } from "./add";

const server = freshServer();

test("B3 Complete moves a task below the open ones", async ({ page }) => {
  await addTask(page, server.url(), "First task");
  await addTask(page, server.url(), "Second task");
  await page.goto(server.url() + routes.index);
  await row(page, "Second task").getByRole("button", { name: "Complete" }).click();
  await expect(row(page, "Second task").getByRole("button", { name: "Complete" })).toHaveCount(0);
  const text = await page.locator("body").innerText();
  expect(text.indexOf("First task")).toBeGreaterThanOrEqual(0);
  expect(text.indexOf("Second task")).toBeGreaterThan(text.indexOf("First task"));
});
