import Link from "next/link";
import NewTaskPage from "../../../../src/app/tasks/new/page";
import createTask from "../../../../src/server/tasks/create_task";
import { redirect } from "next/navigation";

const state = { title: "" };

vi.mock("../../../../src/server/tasks/create_task", () => ({ default: vi.fn() }));
vi.mock("next/navigation", () => ({ redirect: vi.fn() }));
// The title form keeps the title in state; the test sets it.
vi.mock("react", async (original) => ({
  ...(await original<object>()),
  useState: () => [state.title, () => {}],
}));

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

beforeEach(() => {
  state.title = "";
  vi.mocked(createTask).mockReset();
  vi.mocked(redirect).mockReset();
});

it("[case:adds] submitting a title adds the task and shows the list with it", async () => {
  state.title = "Buy milk";
  const { nodes } = render(NewTaskPage());
  const form = nodes.find((n) => n.tag === "form");
  const data = new FormData();
  data.set("title", "Buy milk");
  await (form?.props.action as (d: FormData) => Promise<void>)(data);
  expect(createTask).toHaveBeenCalledWith({ title: "Buy milk" });
  expect(redirect).toHaveBeenCalledWith("/tasks");
});

it("[case:required] the Add button is disabled while the title is empty", () => {
  const button = () => render(NewTaskPage()).nodes.find((n) => n.tag === "button" && n.text === "Add");
  expect(button()?.props.disabled).toBe(true);
  state.title = "x";
  expect(button()?.props.disabled).toBe(false);
});
