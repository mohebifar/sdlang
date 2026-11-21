import Link from "next/link";
import { redirect } from "next/navigation";
import listTasks from "../../server/tasks/list_tasks";
import completeTask from "../../server/tasks/complete_task";
import TaskRow from "./_parts/task_row";

// The list reads the in-memory store, so it renders on every request.
export const dynamic = "force-dynamic";

export default async function TasksPage() {
  const tasks = await listTasks({});

  async function complete(formData: FormData) {
    "use server";
    await completeTask({ id: String(formData.get("id")) });
    redirect("/tasks");
  }

  return (
    <main>
      <h1>Tasks</h1>
      <p>
        <Link href="/tasks/new">New task</Link>
      </p>
      {tasks.length === 0 ? (
        <p>
          No tasks yet. <Link href="/tasks/new">Add a task</Link>
        </p>
      ) : (
        <ul>
          {tasks.map((task) => (
            <TaskRow key={task.id} task={task} complete={complete} />
          ))}
        </ul>
      )}
      <footer>
        <Link href="/about">About</Link>
      </footer>
    </main>
  );
}
