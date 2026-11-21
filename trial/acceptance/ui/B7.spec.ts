import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";

const server = freshServer();

test("B7 the list's footer links to the About page", async ({ page }) => {
  await page.goto(server.url() + routes.index);
  await expect(page.locator(`a[href="${routes.about}"]`).first()).toBeVisible();
});
