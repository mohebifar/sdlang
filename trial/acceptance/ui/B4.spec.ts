import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";
import { row } from "./add";

const server = freshServer();

test("B4 the Add button is disabled for an empty title; adding lands on the list showing the task", async ({ page }) => {
  await page.goto(server.url() + routes.new);
  const add = page.getByRole("button", { name: "Add" });
  await expect(add).toBeDisabled();
  await page.getByRole("textbox").first().fill("Water plants");
  await expect(add).toBeEnabled();
  await add.click();
  await expect(page).toHaveURL(server.url() + routes.index);
  await expect(row(page, "Water plants")).toBeVisible();
});
