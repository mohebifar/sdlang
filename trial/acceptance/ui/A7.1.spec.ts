import { expect, test } from "@playwright/test";
import { freshServer } from "./server";
import { addTask, row } from "./add";

const server = freshServer();

test("A7.1 /todo lists tasks; /tasks is not the list", async ({ page }) => {
  await addTask(page, server.url(), "Moved list");
  await page.goto(server.url() + "/todo");
  await expect(row(page, "Moved list")).toBeVisible();
  const response = await page.goto(server.url() + "/tasks");
  const servedAtTasks = new URL(page.url()).pathname === "/tasks" && response?.status() !== 404;
  expect(servedAtTasks && (await page.getByText("Moved list").count()) > 0).toBe(false);
});
