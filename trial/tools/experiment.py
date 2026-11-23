"""A from-scratch run of one arm: the agent builds the task tracker from the arm's spec in
an empty scaffold, then makes docs/trial.md's ten edits on its own result. The acceptance tests
run after every step. This is not the pinned trial (docs/trial.md starts every arm from the
hand-written baseline); it reuses the trial's spec, edits, agents and acceptance tests.

Usage:
  python3 experiment.py run <language | strong> <root> [--from <step>] [--to <step>]
  python3 experiment.py report <root>

<root> holds one directory per arm (the workspace) and operator/: logs/<arm>/<step>/ (agent
and reviewer session records, plans, check output), results/<arm>.json (one entry per step)
and each arm's copy of the acceptance tests. Agents cannot read operator/ or another arm.
Steps are build, E1 ... E10.
"""
import json
import os
import shutil
import signal
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
TRIAL = os.path.dirname(HERE)
REPO = os.path.dirname(TRIAL)
SPEC = os.path.join(REPO, "cli", "target", "release", "spec")
NODE_BIN = os.path.expanduser("~/.nvm/versions/node/v22.15.1/bin")
STEPS = ["build"] + [f"E{i}" for i in range(1, 11)]
ARMS = ["language", "strong"]
STEP_CAP = {"build": 90 * 60}  # seconds; edits have docs/trial.md's 45 minutes
EDIT_CAP = 45 * 60
GIT = ["git", "-c", "user.name=operator", "-c", "user.email=operator@localhost", "-c", "commit.gpgsign=false"]
BUILD_OUTPUT = [".next", "next-env.d.ts", "tsconfig.tsbuildinfo"]

STRONG_BUILD = """The spec in docs/specs/ describes this app, and none of it is implemented yet: the \
project holds only the toolchain files and the root layout. Implement the app to match the \
spec, following its conventions, with the tests they describe, and keep the project's tests, \
typecheck and build passing."""
STRONG_EDIT = """The spec in docs/specs/ changed; the diff is below. Change the code to match, keep all \
other behaviour, and keep the project's tests passing."""


def env():
    e = dict(os.environ)
    e["PATH"] = NODE_BIN + os.pathsep + e["PATH"]
    return e


def sh(argv, cwd, timeout=None, input=None, extra=None, log=None):
    """Runs argv in its own process group; returns (code, output). Code -9 is a timeout."""
    e = env()
    e.update(extra or {})
    p = subprocess.Popen(argv, cwd=cwd, env=e, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT, text=True, start_new_session=True)
    try:
        out, _ = p.communicate(input=input, timeout=timeout)
        code = p.returncode
    except subprocess.TimeoutExpired:
        os.killpg(p.pid, signal.SIGKILL)
        out, _ = p.communicate()
        code = -9
    if log:
        with open(log, "a") as f:
            f.write(f"$ {' '.join(argv)}  (exit {code})\n{out}\n")
    return code, out


def head(ws):
    return subprocess.run(["git", "rev-parse", "HEAD"], cwd=ws, capture_output=True, text=True).stdout.strip()


def commit(ws, message, paths=("-A",)):
    subprocess.run(["git", "add", *paths], cwd=ws, check=True)
    subprocess.run(GIT + ["commit", "-q", "--allow-empty", "-m", message], cwd=ws, check=True)


def clean_build_output(ws):
    for p in BUILD_OUTPUT:
        path = os.path.join(ws, p)
        if os.path.isdir(path):
            shutil.rmtree(path, ignore_errors=True)
        elif os.path.exists(path):
            os.remove(path)


def project_checks(ws, log):
    """The project's own tests, typecheck and build, as the operator runs them."""
    results = {}
    for name, argv in [("test", ["pnpm", "test"]), ("typecheck", ["pnpm", "typecheck"]), ("build", ["pnpm", "build"])]:
        code, out = sh(argv, ws, timeout=900, log=log)
        results[name] = {"passed": code == 0, "tail": "\n".join(out.strip().splitlines()[-40:])}
    clean_build_output(ws)
    return results


def sessions(logdir):
    """Totals over the agent and reviewer session records Claude Code wrote."""
    tot = {"sessions": 0, "agent_sessions": 0, "reviewer_sessions": 0, "cost_usd": 0.0,
           "input_tokens": 0, "output_tokens": 0, "cache_read_tokens": 0, "cache_write_tokens": 0,
           "turns": 0, "errors": 0, "refused": 0}
    if not os.path.isdir(logdir):
        return tot
    for name in sorted(os.listdir(logdir)):
        if not (name.startswith("agent-") or name.startswith("reviewer-")) or not name.endswith(".json"):
            continue
        try:
            d = json.load(open(os.path.join(logdir, name)))
        except (ValueError, OSError):
            tot["errors"] += 1
            continue
        tot["sessions"] += 1
        tot["agent_sessions" if name.startswith("agent-") else "reviewer_sessions"] += 1
        tot["cost_usd"] += d.get("total_cost_usd") or 0
        tot["turns"] += d.get("num_turns") or 0
        u = d.get("usage") or {}
        tot["input_tokens"] += u.get("input_tokens", 0)
        tot["output_tokens"] += u.get("output_tokens", 0)
        tot["cache_read_tokens"] += u.get("cache_read_input_tokens", 0)
        tot["cache_write_tokens"] += u.get("cache_creation_input_tokens", 0)
        if d.get("is_error"):
            tot["errors"] += 1
        # A tool call Claude Code refused (headless, an unapproved ask is a refusal): the
        # session's own report says so; permission_denials is not always filled in.
        text = (d.get("result") or "").lower()
        if d.get("permission_denials") or "haven't granted" in text or "permission" in text and "denied" in text:
            tot["refused"] += 1
    tot["cost_usd"] = round(tot["cost_usd"], 4)
    return tot


def changed_files(ws, before, after):
    out = subprocess.run(["git", "diff", "--name-only", before, after], cwd=ws, capture_output=True, text=True).stdout
    skip = ("spec/", ".spec/", "docs/specs/", "plan.json", "sd.lock", "spec-packages/")
    return [f for f in out.split() if not f.startswith(skip)]


def acceptance(root, arm, ws, step, logdir):
    """Runs the acceptance tests active at the step, from this arm's own copy of acceptance/."""
    copy = os.path.join(root, "operator", f"acceptance-{arm}")
    if not os.path.isdir(copy):
        src = os.path.join(TRIAL, "acceptance")
        shutil.copytree(src, copy, ignore=shutil.ignore_patterns("node_modules", "test-results"))
        os.symlink(os.path.join(src, "node_modules"), os.path.join(copy, "node_modules"))
    edit = "baseline" if step == "build" else step
    out = os.path.join(logdir, "acceptance.json")
    extra = {}
    patched = os.path.join(root, "operator", f"{arm}-about.patched.tsx")
    if os.path.exists(patched):
        extra["PATCHED_ABOUT"] = patched
    sh(["node", "run.mjs", ws, edit, "--out", out], copy, timeout=1800, extra=extra,
       log=os.path.join(logdir, "acceptance.log"))
    try:
        return json.load(open(out))
    except (ValueError, OSError):
        return {"build": "not run", "results": {}}


def setup(root, arm):
    ws = os.path.join(root, arm)
    code, out = sh(["sh", os.path.join(HERE, "make-arm.sh"), arm, ws, "scratch"], TRIAL, timeout=900)
    if code != 0:
        sys.exit(f"make-arm failed:\n{out}")
    if arm == "language":
        # The scaffold's toolchain files are the stack's outputs; the plan acquires them.
        code, out = sh([SPEC, "import", "package.json", "pnpm-lock.yaml", "tsconfig.json", "next.config.ts", "--accept"], ws)
        if code != 0:
            sys.exit(f"sdc import failed:\n{out}")
        commit(ws, "accept sdc import")
    return ws


def other_arms(root, arm):
    return ":".join([os.path.join(root, a) for a in ARMS if a != arm] + [os.path.join(root, "operator")])


def hand_patch_e8(root, arm, ws, logdir):
    """The E8 hand patch. Where the script cannot place it, the run stops so the operator
    patches by hand, commits the page, saves the copy and resumes with E8_PATCHED=1."""
    if os.environ.get("E8_PATCHED") == "1":
        return True, "patched by hand"
    code, out = sh(["node", os.path.join(TRIAL, "acceptance", "patches", "e8.mjs"), ws,
                    os.path.join(root, "operator", f"{arm}-about.patched.tsx")], ws, log=os.path.join(logdir, "operator.log"))
    if code != 0:
        sys.exit(f"[{arm}] E8 hand patch needs the operator: {out.strip()}")
    commit(ws, "hand patch for E8", ("src/app/about/page.tsx",))
    return True, out


def language_step(root, ws, step, logdir, cap):
    log = os.path.join(logdir, "operator.log")
    extra = {"TRIAL_LOG_DIR": logdir, "TRIAL_DENY_READ": other_arms(root, "language")}
    info = {}
    start = time.time()
    if step != "build":
        if step == "E8":
            ok, out = hand_patch_e8(root, "language", ws, logdir)
            info["hand_patch"] = "applied" if ok else out.strip()
        code, out = sh(["python3", os.path.join(HERE, "spec_edit.py"), step, ws], ws, log=log)
        if code != 0:
            return {"error": f"spec edit failed: {out}"}
        commit(ws, f"spec: {step}", ("-A", "spec"))
    rounds = []
    # One plan and apply; at most two more when obligations are left over (the build's
    # acquisition of the stack, or verify and review a first apply did not discharge).
    for r in range(3):
        remaining = cap - (time.time() - start)
        if remaining <= 0:
            info["timed_out"] = True
            break
        argv = [SPEC, "plan", "--id", f"p-{step}-{r + 1}"]
        if step == "E8" and r == 0 and info.get("hand_patch") == "applied":
            argv += ["--adopt", "src/app/about/page.tsx"]
        code, plan = sh(argv, ws, log=log)
        with open(os.path.join(logdir, f"plan-{r + 1}.txt"), "w") as f:
            f.write(plan)
        if code != 0:
            rounds.append({"plan": "failed"})
            break
        if "Changes: 0 create, 0 update" in plan and "0 obligation-only" in plan and "Obligations" not in plan:
            rounds.append({"plan": "empty"})
            break
        code, out = sh([SPEC, "apply", "--agent", os.path.join(HERE, "agent.sh"),
                        "--reviewer", os.path.join(HERE, "reviewer.sh")],
                       ws, timeout=remaining, extra=extra, log=log)
        with open(os.path.join(logdir, f"apply-{r + 1}.txt"), "w") as f:
            f.write(out)
        rounds.append({"apply_exit": code,
                       "published": out.count(" published "),
                       "failed": sum(1 for l in out.splitlines() if l.startswith("tx-") and "published" not in l)})
        if code == -9:
            info["timed_out"] = True
            break
        if code != 0:
            break  # a failed transaction already had its retries inside sdc apply
        code, check = sh([SPEC, "check"], ws, log=log)
        if code == 0 and "consistent" in check:
            break
    code, check = sh([SPEC, "check"], ws, log=log)
    info["spec_check"] = check.strip().splitlines()[-1] if check.strip() else ""
    info["rounds"] = rounds
    info["wall_s"] = round(time.time() - start)
    # Whatever is left uncommitted (the plan document) goes into the step's commit.
    commit(ws, f"operator: {step} done")
    return info


def strong_step(root, ws, step, logdir, cap):
    log = os.path.join(logdir, "operator.log")
    extra = {"TRIAL_LOG_DIR": logdir, "TRIAL_DENY_READ": other_arms(root, "strong")}
    info = {"attempts": []}
    start = time.time()
    if step == "build":
        instruction = STRONG_BUILD
    else:
        if step == "E8":
            ok, out = hand_patch_e8(root, "strong", ws, logdir)
            info["hand_patch"] = "applied" if ok else out.strip()
        code, out = sh(["python3", os.path.join(HERE, "docs_edit.py"), step, ws], ws, log=log)
        if code != 0:
            return {"error": f"docs edit failed: {out}"}
        commit(ws, f"spec: {step}", ("-A", "docs"))
        diff = subprocess.run(["git", "diff", "HEAD~1", "HEAD", "--", "docs"], cwd=ws, capture_output=True, text=True).stdout
        instruction = f"{STRONG_EDIT}\n\n```diff\n{diff}```\n"
    prompt = instruction
    for attempt in range(3):  # the first try and up to two retries
        remaining = cap - (time.time() - start)
        if remaining <= 0:
            info["timed_out"] = True
            break
        code, out = sh([os.path.join(HERE, "strong-agent.sh")], ws, timeout=remaining, input=prompt, extra=extra, log=log)
        if code == -9:
            info["timed_out"] = True
            info["attempts"].append({"agent_exit": code})
            break
        checks = project_checks(ws, os.path.join(logdir, f"checks-{attempt + 1}.log"))
        info["attempts"].append({"agent_exit": code, "checks": {k: v["passed"] for k, v in checks.items()}})
        failed = {k: v for k, v in checks.items() if not v["passed"]}
        if not failed:
            break
        report = "\n\n".join(f"`pnpm {k}` failed:\n```\n{v['tail']}\n```" for k, v in failed.items())
        prompt = f"{instruction}\n\nA previous attempt at this left the project failing these checks:\n\n{report}\n"
    clean_build_output(ws)
    info["wall_s"] = round(time.time() - start)
    commit(ws, f"agent: {step}")
    return info


def run(root, arm, first, last):
    root = os.path.abspath(root)
    os.makedirs(root, exist_ok=True)
    ws = os.path.join(root, arm)
    results_path = os.path.join(root, "operator", "results", f"{arm}.json")
    os.makedirs(os.path.dirname(results_path), exist_ok=True)
    results = json.load(open(results_path)) if os.path.exists(results_path) else {}
    if not os.path.isdir(ws):
        ws = setup(root, arm)
    passed_before, counted = {}, set()
    for s in STEPS[:STEPS.index(first)]:
        counted.update(results.get(s, {}).get("regressions", []))
        for t, v in results.get(s, {}).get("acceptance", {}).items():
            passed_before[t] = v == "passed" or passed_before.get(t, False)
    for step in STEPS[STEPS.index(first):STEPS.index(last) + 1]:
        logdir = os.path.join(root, "operator", "logs", arm, step)
        if os.path.isdir(logdir):
            shutil.rmtree(logdir)
        os.makedirs(logdir)
        before = head(ws)
        cap = STEP_CAP.get(step, EDIT_CAP)
        print(f"[{arm}] {step} started {time.strftime('%H:%M:%S')}", flush=True)
        info = (language_step if arm == "language" else strong_step)(root, ws, step, logdir, cap)
        after = head(ws)
        acc = acceptance(root, arm, ws, step, logdir)
        own = project_checks(ws, os.path.join(logdir, "project-checks.log"))
        new_ids = [] if step == "build" else [t for t in acc.get("results", {}) if t.startswith(f"A{step[1:]}.")]
        base_ids = [t for t in acc.get("results", {}) if t.startswith("B")] if step == "build" else new_ids
        res = acc.get("results", {})
        # Counted once per test, at the first step where it fails after having passed.
        regressions = [t for t, v in res.items()
                       if t not in base_ids and v != "passed" and passed_before.get(t) and t not in counted]
        counted.update(regressions)
        for t, v in res.items():
            passed_before[t] = passed_before.get(t, False) or v == "passed"
        entry = {
            "step": step,
            "info": info,
            "intended": all(res.get(t) == "passed" for t in base_ids) and bool(base_ids),
            "target_tests": {t: res.get(t) for t in base_ids},
            "regressions": regressions,
            "acceptance": res,
            "acceptance_build": acc.get("build"),
            "project_checks": {k: v["passed"] for k, v in own.items()},
            "files_changed": changed_files(ws, before, after),
            "sessions": sessions(logdir),
        }
        results[step] = entry
        json.dump(results, open(results_path, "w"), indent=2)
        print(f"[{arm}] {step} done: intended={entry['intended']} regressions={regressions} "
              f"cost=${entry['sessions']['cost_usd']} wall={info.get('wall_s')}s refused={entry['sessions']['refused']}", flush=True)


def report(root):
    rows = []
    for arm in ARMS:
        p = os.path.join(root, "operator", "results", f"{arm}.json")
        if not os.path.exists(p):
            continue
        res = json.load(open(p))
        for step in STEPS:
            e = res.get(step)
            if not e:
                continue
            s = e["sessions"]
            rows.append((arm, step, e["intended"], len(e["regressions"]), ",".join(e["regressions"]),
                         s["cost_usd"], s["input_tokens"] + s["cache_read_tokens"] + s["cache_write_tokens"],
                         s["output_tokens"], e["info"].get("wall_s"), len(e["files_changed"])))
    print("arm       step   intended  regr  cost$    in_tok     out_tok  wall_s  files")
    for r in rows:
        print(f"{r[0]:9} {r[1]:6} {str(r[2]):9} {r[3]:<5} {r[5]:<8} {r[6]:<10} {r[7]:<8} {r[8]!s:<7} {r[9]}  {r[4]}")


if __name__ == "__main__":
    if len(sys.argv) >= 4 and sys.argv[1] == "run":
        a = sys.argv[4:]
        first = a[a.index("--from") + 1] if "--from" in a else "build"
        last = a[a.index("--to") + 1] if "--to" in a else "E10"
        run(sys.argv[3], sys.argv[2], first, last)
    elif len(sys.argv) == 3 and sys.argv[1] == "report":
        report(sys.argv[2])
    else:
        sys.exit(__doc__)
