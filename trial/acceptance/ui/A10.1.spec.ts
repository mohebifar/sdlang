import { expect, test } from "@playwright/test";
import { freshServer } from "./server";

const server = freshServer();

test("A10.1 /about responds 404", async ({ page }) => {
  const response = await page.goto(server.url() + "/about");
  expect(response?.status()).toBe(404);
});
