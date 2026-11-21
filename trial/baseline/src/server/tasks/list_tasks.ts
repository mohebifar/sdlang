import { all } from "../../lib/tasks/store";

export type Input = Record<string, never>;
export type Task = { id: string; title: string; done: boolean; created_at: string };

/** Lists all tasks, open ones first, each group newest first. */
export default async function listTasks(_input: Input = {}): Promise<Task[]> {
  const tasks = all<Task>();
  return [...tasks.filter((t) => !t.done), ...tasks.filter((t) => t.done)].map((t) => ({
    id: t.id,
    title: t.title,
    done: t.done,
    created_at: t.created_at,
  }));
}
