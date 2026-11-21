// Adding a task through the UI: the new task page's one text field and its Add button.
import { expect, type Page } from "@playwright/test";
import { routes } from "./server";

export async function addTask(page: Page, base: string, title: string) {
  await page.goto(base + routes.new);
  await page.getByRole("textbox").first().fill(title);
  await page.getByRole("button", { name: "Add" }).click();
  await expect(page).toHaveURL(base + routes.index);
}

/** The list item (or table row) holding the task with this title. */
export function row(page: Page, title: string) {
  return page.locator("li, tr").filter({ hasText: title });
}
