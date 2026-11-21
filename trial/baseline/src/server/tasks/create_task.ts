import { insert } from "../../lib/tasks/store";

export type Input = { title: string };
export type Output = { id: string; title: string; done: boolean; created_at: string };

/** Adds a task. */
export default async function createTask(input: Input): Promise<Output> {
  return insert<Output>({
    id: crypto.randomUUID(),
    title: input.title,
    done: false,
    created_at: new Date().toISOString(),
  });
}
