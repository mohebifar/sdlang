import { expect, test } from "@playwright/test";
import { freshServer } from "./server";
import { addTask } from "./add";

const server = freshServer();

test("A7.2 adding a task lands on /todo", async ({ page }) => {
  await addTask(page, server.url(), "Lands on todo");
  await expect(page).toHaveURL(server.url() + "/todo");
});
