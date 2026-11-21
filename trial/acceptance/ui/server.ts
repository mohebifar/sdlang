// A fresh `next start` on the arm's build for one test file, and the route map.
import { spawn, type ChildProcess } from "node:child_process";
import net from "node:net";
import path from "node:path";
import { test } from "@playwright/test";

const arm = process.env.ARM;
if (!arm) throw new Error("set ARM to the arm's project directory");

/** The operator's route map for the current edit (TRIAL.md: the index moves at E7). */
export const routes: { index: string; new: string; about: string } = JSON.parse(
  process.env.ROUTES ?? '{"index":"/tasks","new":"/tasks/new","about":"/about"}',
);

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const s = net.createServer();
    s.listen(0, "127.0.0.1", () => {
      const port = (s.address() as net.AddressInfo).port;
      s.close(() => resolve(port));
    });
    s.on("error", reject);
  });
}

/** Starts the server before the file's tests and stops it after; returns its base URL. */
export function freshServer(): { url: () => string } {
  let child: ChildProcess | undefined;
  let base = "";
  test.beforeAll(async () => {
    const port = await freePort();
    base = `http://127.0.0.1:${port}`;
    const next = path.join(arm!, "node_modules", "next", "dist", "bin", "next");
    child = spawn(process.execPath, [next, "start", "-p", String(port), "-H", "127.0.0.1"], {
      cwd: arm,
      stdio: "ignore",
      env: { ...process.env, NODE_ENV: "production" },
    });
    const deadline = Date.now() + 30_000;
    for (;;) {
      try {
        await fetch(base + "/");
        break;
      } catch {
        if (Date.now() > deadline) throw new Error("next start did not answer within 30s");
        await new Promise((r) => setTimeout(r, 200));
      }
    }
  });
  test.afterAll(() => {
    child?.kill("SIGKILL");
  });
  return { url: () => base };
}

/** Today's date as YYYY-MM-DD (UTC, as the app formats dates). */
export const today = () => new Date().toISOString().slice(0, 10);
