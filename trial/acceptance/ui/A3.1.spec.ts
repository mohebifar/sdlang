import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";
import { addTask, row } from "./add";

const server = freshServer();

test("A3.1 the checkbox marks a task done and then open again", async ({ page }) => {
  await addTask(page, server.url(), "Toggle me");
  await page.goto(server.url() + routes.index);
  const box = () => row(page, "Toggle me").getByRole("checkbox");
  await expect(box()).not.toBeChecked();
  await box().click();
  await expect(box()).toBeChecked();
  await page.reload();
  await expect(box()).toBeChecked();
  await box().click();
  await expect(box()).not.toBeChecked();
  await page.reload();
  await expect(box()).not.toBeChecked();
});
