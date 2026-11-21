import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";

const server = freshServer();

test("A7.3 the About page links to /todo", async ({ page }) => {
  await page.goto(server.url() + routes.about);
  await expect(page.locator('a[href="/todo"]').first()).toBeVisible();
});
