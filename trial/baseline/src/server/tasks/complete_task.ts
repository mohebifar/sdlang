import { find, save } from "../../lib/tasks/store";

export type Input = { id: string };
export type Output = { id: string; done: boolean };

type Task = { id: string; title: string; done: boolean; created_at: string };

/** A declared error: rejects with an Error whose code is the error's key. */
function notFound(): Error {
  return Object.assign(new Error("No task has that id"), { code: "not_found" });
}

/** Marks a task done. */
export default async function completeTask(input: Input): Promise<Output> {
  const task = find<Task>(input.id);
  if (!task) {
    throw notFound();
  }
  save({ ...task, done: true });
  return { id: task.id, done: true };
}
