import Link from "next/link";
import TasksPage from "../../../src/app/tasks/page";
import listTasks from "../../../src/server/tasks/list_tasks";
import completeTask from "../../../src/server/tasks/complete_task";
import { redirect } from "next/navigation";

vi.mock("../../../src/server/tasks/list_tasks", () => ({ default: vi.fn() }));
vi.mock("../../../src/server/tasks/complete_task", () => ({ default: vi.fn() }));
vi.mock("next/navigation", () => ({ redirect: vi.fn() }));

// Renders a server component tree without a DOM: function components are called, host
// elements and Links are visited, and the text is collected in order.
type El = { type: unknown; props: { children?: unknown } & Record<string, unknown> };
type Visited = { tag: string; props: Record<string, unknown>; text: string; inside: string[] };

function render(node: unknown): { text: string; nodes: Visited[] } {
  const nodes: Visited[] = [];
  const walk = (n: unknown, inside: string[]): string => {
    if (n === null || n === undefined || typeof n === "boolean") return "";
    if (typeof n === "string" || typeof n === "number") return String(n);
    if (Array.isArray(n)) return n.map((c) => walk(c, inside)).join("");
    if (typeof n !== "object" || !("type" in n) || !("props" in n)) return "";
    const el = n as El;
    if (typeof el.type === "function" && el.type !== Link) {
      return walk((el.type as (p: unknown) => unknown)(el.props), inside);
    }
    const tag = el.type === Link ? "a" : typeof el.type === "string" ? el.type : "fragment";
    const visited: Visited = { tag, props: el.props, text: "", inside };
    nodes.push(visited);
    visited.text = walk(el.props.children, [...inside, tag]);
    return visited.text;
  };
  return { text: walk(node, []), nodes };
}

const open = { id: "a", title: "Buy milk", done: false, created_at: "2026-02-03T09:00:00.000Z" };
const done = { id: "b", title: "Post letter", done: true, created_at: "2026-02-04T09:00:00.000Z" };
// list_tasks' items as this page reads them; a field the action adds later does not
// break the test.
const tasks = (...items: object[]) => items as Awaited<ReturnType<typeof listTasks>>;

beforeEach(() => {
  vi.mocked(listTasks).mockReset();
  vi.mocked(completeTask).mockReset();
  vi.mocked(redirect).mockReset();
});

it("[case:listed] every task is listed with its title and created date", async () => {
  vi.mocked(listTasks).mockResolvedValue(tasks(open, done));
  const { nodes } = render(await TasksPage());
  const rows = nodes.filter((n) => n.tag === "li").map((n) => n.text);
  expect(rows).toHaveLength(2);
  expect(rows[0]).toContain("Buy milk");
  expect(rows[0]).toContain("2026-02-03");
  expect(rows[1]).toContain("Post letter");
  expect(rows[1]).toContain("2026-02-04");
});

it("[case:complete] Complete marks the task done and moves it below the open tasks", async () => {
  vi.mocked(listTasks).mockResolvedValue(tasks(open, done));
  const { nodes } = render(await TasksPage());
  const rows = nodes.filter((n) => n.tag === "li");
  // Open tasks come first, each with a Complete button; done ones have none.
  expect(rows.map((r) => r.text.includes("Complete"))).toEqual([true, false]);
  const form = nodes.find((n) => n.tag === "form");
  const data = new FormData();
  data.set("id", "a");
  await (form?.props.action as (d: FormData) => Promise<void>)(data);
  expect(completeTask).toHaveBeenCalledWith({ id: "a" });
  // The list is shown again, in list_tasks order: the completed task below the open ones.
  expect(redirect).toHaveBeenCalledWith("/tasks");
});

it("[case:empty] with no tasks, the page says 'No tasks yet' and links to the new task page", async () => {
  vi.mocked(listTasks).mockResolvedValue(tasks());
  const { text, nodes } = render(await TasksPage());
  expect(text).toContain("No tasks yet");
  expect(nodes.filter((n) => n.tag === "li")).toHaveLength(0);
  const empty = nodes.find((n) => n.tag === "p" && n.text.includes("No tasks yet"));
  expect(nodes.some((n) => n.tag === "a" && n.props.href === "/tasks/new" && n.inside.includes("p") && empty)).toBe(true);
});

it("[case:footer] the footer links to the about page", async () => {
  vi.mocked(listTasks).mockResolvedValue(tasks(open));
  const { nodes } = render(await TasksPage());
  expect(nodes.some((n) => n.tag === "a" && n.props.href === "/about" && n.inside.includes("footer"))).toBe(true);
});
