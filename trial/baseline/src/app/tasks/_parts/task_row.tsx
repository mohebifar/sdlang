import { formatDate } from "../../../lib/tasks/dates";

export type RowTask = { id: string; title: string; done: boolean; created_at: string };

/** One task: its title, its created date and, while open, a Complete button. */
export default function TaskRow({
  task,
  complete,
}: {
  task: RowTask;
  complete: (formData: FormData) => Promise<void>;
}) {
  return (
    <li>
      <span>{task.title}</span> <time dateTime={task.created_at}>{formatDate(task.created_at)}</time>
      {task.done ? (
        <span> Done</span>
      ) : (
        <form action={complete}>
          <input type="hidden" name="id" value={task.id} />
          <button type="submit">Complete</button>
        </form>
      )}
    </li>
  );
}
