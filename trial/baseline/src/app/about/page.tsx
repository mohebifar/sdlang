import Link from "next/link";

export default function AboutPage() {
  return (
    <main>
      <h1>About</h1>
      <p>This is a small task tracker: add tasks, see them listed, and complete them.</p>
      <p>
        It keeps tasks in memory, so they are gone after a restart.{" "}
        <Link href="/tasks">Back to the tasks</Link>
      </p>
    </main>
  );
}
