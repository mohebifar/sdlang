import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";

const server = freshServer();

test("B2 with no tasks the list says 'No tasks yet' and links to the new task page", async ({ page }) => {
  await page.goto(server.url() + routes.index);
  await expect(page.getByText("No tasks yet")).toBeVisible();
  await expect(page.locator(`a[href="${routes.new}"]`).first()).toBeVisible();
});
