import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";

const server = freshServer();

test("A9.1 an 81-character title shows the message and adds nothing", async ({ page }) => {
  const title = "x".repeat(81);
  await page.goto(server.url() + routes.new);
  await page.getByRole("textbox").first().fill(title);
  const add = page.getByRole("button", { name: "Add" });
  if (await add.isEnabled()) {
    await add.click();
  }
  await expect(page.getByText("Keep it under 80 characters")).toBeVisible();
  await page.goto(server.url() + routes.index);
  await expect(page.getByText(title)).toHaveCount(0);
});
