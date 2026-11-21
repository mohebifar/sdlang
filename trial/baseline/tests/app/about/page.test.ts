import Link from "next/link";
import AboutPage from "../../../src/app/about/page";

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

it("[case:explains] the page has the heading 'About' and links to the task list", () => {
  const { nodes } = render(AboutPage());
  expect(nodes.filter((n) => n.tag === "h1").map((n) => n.text)).toEqual(["About"]);
  expect(nodes.some((n) => n.tag === "a" && n.props.href === "/tasks")).toBe(true);
});
