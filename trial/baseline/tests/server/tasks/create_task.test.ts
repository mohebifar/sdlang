import createTask from "../../../src/server/tasks/create_task";
import { all, find } from "../../../src/lib/tasks/store";

beforeEach(() => {
  delete (globalThis as { __tasksStore?: unknown }).__tasksStore;
});

it("[case:stored] a created task is stored with its title, not done", async () => {
  const task = await createTask({ title: "Buy milk" });
  expect(task).toMatchObject({ title: "Buy milk", done: false });
  expect(task.id).toBeTruthy();
  expect(Number.isNaN(Date.parse(task.created_at))).toBe(false);
  expect(find(task.id)).toEqual(task);
  expect(all().map((t) => t.id)).toEqual([task.id]);
});
