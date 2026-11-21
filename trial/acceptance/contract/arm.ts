// An arm's action modules, by their adapter paths (src/server/tasks/<name>.ts), and the
// declared-error convention: a declared error rejects with an Error whose code is its key.
import fs from "node:fs";
import path from "node:path";

const arm = process.env.ARM;
if (!arm) throw new Error("set ARM to the arm's project directory");
export const armDir = arm;

type Fn = (input: unknown) => Promise<any>;

export function has(name: string): boolean {
  return fs.existsSync(path.join(arm!, "src/server/tasks", `${name}.ts`));
}

export async function action(name: string): Promise<Fn> {
  const mod = await import(path.join(arm!, "src/server/tasks", `${name}.ts`));
  return mod.default as Fn;
}

/** Marks a task done with whichever action the arm has: complete_task before E3, then set_task_done. */
export async function markDone(id: string): Promise<void> {
  if (has("set_task_done")) {
    await (await action("set_task_done"))({ id, done: true });
  } else {
    await (await action("complete_task"))({ id });
  }
}

/** Distinct created_at timestamps, so "newest first" is never a tie. */
export const tick = () => new Promise((r) => setTimeout(r, 5));
