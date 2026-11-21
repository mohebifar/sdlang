import { expect, test } from "@playwright/test";
import { freshServer, routes } from "./server";
import { addTask, row } from "./add";

const server = freshServer();

test("A4.2 a title typed with extra spaces is listed trimmed", async ({ page }) => {
  await addTask(page, server.url(), "  Buy   milk ");
  await page.goto(server.url() + routes.index);
  await expect(row(page, "Buy milk")).toBeVisible();
  const text = await row(page, "Buy milk").first().textContent();
  expect(text).not.toContain("Buy   milk");
});
