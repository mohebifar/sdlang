import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";

const server = freshServer();

test("B6 the About page has the heading 'About' and links to the list", async ({ page }) => {
  await page.goto(server.url() + routes.about);
  await expect(page.getByRole("heading", { name: "About" }).first()).toBeVisible();
  await expect(page.locator(`a[href="${routes.index}"]`).first()).toBeVisible();
});
