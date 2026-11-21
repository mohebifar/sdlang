import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";
import { addTask, row } from "./add";

const server = freshServer();

test("A1.1 Delete removes a task from the list", async ({ page }) => {
  await addTask(page, server.url(), "Keep me");
  await addTask(page, server.url(), "Delete me");
  await page.goto(server.url() + routes.index);
  await row(page, "Delete me").getByRole("button", { name: "Delete" }).click();
  await expect(page.getByText("Delete me")).toHaveCount(0);
  await page.reload();
  await expect(page.getByText("Delete me")).toHaveCount(0);
  await expect(row(page, "Keep me")).toBeVisible();
});
