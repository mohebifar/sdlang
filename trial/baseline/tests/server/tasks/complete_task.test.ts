import completeTask from "../../../src/server/tasks/complete_task";
import { find, insert } from "../../../src/lib/tasks/store";

beforeEach(() => {
  delete (globalThis as { __tasksStore?: unknown }).__tasksStore;
});

it("[case:marks_done] after complete_task the task is done", async () => {
  insert({ id: "t1", title: "Buy milk", done: false, created_at: "2026-01-01T10:00:00.000Z" });
  expect(await completeTask({ id: "t1" })).toEqual({ id: "t1", done: true });
  expect(find<{ id: string; created_at: string; done: boolean }>("t1")?.done).toBe(true);
});

it("[case:not_found] complete_task fails with not_found exactly when no task has that id", async () => {
  await expect(completeTask({ id: "missing" })).rejects.toMatchObject({
    code: "not_found",
    message: "No task has that id",
  });
});
