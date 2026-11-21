import { redirect } from "next/navigation";
import createTask from "../../../server/tasks/create_task";
import TitleForm from "./_parts/title_form";

export default function NewTaskPage() {
  async function add(formData: FormData) {
    "use server";
    await createTask({ title: String(formData.get("title") ?? "") });
    redirect("/tasks");
  }

  return (
    <main>
      <h1>New task</h1>
      <TitleForm add={add} />
    </main>
  );
}
