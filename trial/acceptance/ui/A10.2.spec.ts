import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";

const server = freshServer();

test("A10.2 the list has no link to /about", async ({ page }) => {
  const response = await page.goto(server.url() + routes.index);
  expect(response?.status()).toBe(200);
  await expect(page.locator('a[href="/about"]')).toHaveCount(0);
});
