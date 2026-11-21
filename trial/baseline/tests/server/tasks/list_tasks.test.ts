import listTasks from "../../../src/server/tasks/list_tasks";
import { insert } from "../../../src/lib/tasks/store";

beforeEach(() => {
  delete (globalThis as { __tasksStore?: unknown }).__tasksStore;
});

const task = (id: string, day: number, done: boolean) => ({
  id,
  title: `Task ${id}`,
  done,
  created_at: `2026-01-0${day}T10:00:00.000Z`,
});

it("[case:order] open tasks come before done ones; within each group, newest first", async () => {
  insert(task("open-old", 1, false));
  insert(task("done-old", 2, true));
  insert(task("open-new", 3, false));
  insert(task("done-new", 4, true));
  const ids = (await listTasks({})).map((t) => t.id);
  expect(ids).toEqual(["open-new", "open-old", "done-new", "done-old"]);
});
