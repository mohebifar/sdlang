import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";

const server = freshServer();

test("A8.1 the About page shows 'About this app' and 'Built for the sdlang trial.'", async ({ page }) => {
  await page.goto(server.url() + routes.about);
  await expect(page.getByRole("heading", { name: "About this app" })).toBeVisible();
  await expect(page.getByText("Built for the sdlang trial.")).toBeVisible();
});
