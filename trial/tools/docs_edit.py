"""The strong arm's Markdown change for one edit: the same text as the spec change that
spec_edit.py applies to the language arm (docs/trial.md, The ten edits).

Usage: python3 docs_edit.py <E1..E10> <arm dir>
Each edit is applied to the arm's docs/specs/ after the previous ones; a text that is not
found exactly once stops the script, so nothing is applied by guesswork.
"""
import os
import sys


def rep(path, old, new):
    text = open(path).read()
    if text.count(old) != 1:
        sys.exit(f"{path}: expected exactly one match for {old[:60]!r}")
    open(path, "w").write(text.replace(old, new))


def edit(e, root):
    tasks = os.path.join(root, "docs", "specs", "tasks.md")
    about = os.path.join(root, "docs", "specs", "about.md")
    if e == "E1":
        rep(tasks, "- Exports: `insert`, `all`, `find`, `save`", "- Exports: `insert`, `all`, `find`, `save`, `remove`")
        rep(tasks, "- unknown_id: find returns null for an unknown id\n",
            "- unknown_id: find returns null for an unknown id\n"
            "- removes: after remove, find returns null and all omits the record\n")
        rep(tasks, "## page tasks_index\n", """## action delete_task

Deletes a task.

- Input: `{ id: string }`
- Uses: store
- Errors:
  - not_found: No task has that id

## page tasks_index
""")
        rep(tasks, "- Calls: list_tasks, complete_task\n", "- Calls: list_tasks, complete_task, delete_task\n")
        rep(tasks, "- footer: the footer links to about\n",
            "- footer: the footer links to about\n"
            "- delete: each row has a Delete button that removes the task from the list\n")
    elif e == "E2":
        rep(tasks, "## action create_task\n", """## type TaskStatus

`"open" | "done"`

## action create_task
""")
        rep(tasks, "Lists all tasks, open ones first, each group newest first.\n\n- Input: `{ }`",
            "Lists all tasks, open ones first, each group newest first.\n\n- Input: `{ status: TaskStatus? }`")
        rep(tasks, "- order: open tasks come before done ones; within each group, newest first\n",
            "- order: open tasks come before done ones; within each group, newest first\n"
            "- filter: with a status, only tasks with that status are returned; without one, all\n")
    elif e == "E3":
        rep(tasks, """## action complete_task

Marks a task done.

- Input: `{ id: string }`
- Output: `{ id: string, done: boolean }`
- Uses: store
- Errors:
  - not_found: No task has that id

Expectations:

- marks_done: after complete_task, the stored task is done
""", """## action set_task_done

Marks a task done or open. It replaces complete_task (moved: complete_task is now
set_task_done).

- Input: `{ id: string, done: boolean }`
- Output: `{ id: string, done: boolean }`
- Uses: store
- Errors:
  - not_found: No task has that id

Expectations:

- toggles: set_task_done with done false makes a done task open again
""")
        rep(tasks, "One row per task: title, created date and a Complete button for open tasks.",
            "One row per task: title, created date and a checkbox that toggles done.")
        rep(tasks, "- Calls: list_tasks, complete_task, delete_task\n", "- Calls: list_tasks, set_task_done, delete_task\n")
        rep(tasks, "- complete: Complete marks the task done and moves it below the open tasks\n",
            "- toggle: the checkbox toggles a task between open and done, moving it between the groups\n")
    elif e == "E4":
        rep(tasks, "## action create_task\n\nAdds a task.\n",
            "## action create_task\n\nAdds a task.\n\n"
            "Store the title with surrounding whitespace removed and inner runs of spaces collapsed to one.\n")
    elif e == "E5":
        rep(tasks, "- Output: `[{ id: string, title: string, done: boolean, created_at: timestamp }]`",
            "- Output: `[{ id: string, title: string, done: boolean, created_at: timestamp, done_at: timestamp? }]`")
        rep(tasks, "- filter: with a status, only tasks with that status are returned; without one, all\n",
            "- filter: with a status, only tasks with that status are returned; without one, all\n"
            "- done_at: done_at is set when a task is marked done and null when it is open\n")
        rep(tasks, "- Input: `{ id: string, done: boolean }`\n- Output: `{ id: string, done: boolean }`",
            "- Input: `{ id: string, done: boolean }`\n- Output: `{ id: string, done: boolean, done_at: timestamp? }`")
        rep(tasks, "- toggles: set_task_done with done false makes a done task open again\n",
            "- toggles: set_task_done with done false makes a done task open again\n"
            "- done_at: done_at is set when a task is marked done and null when it is open\n")
    elif e == "E6":
        rep(tasks, "Show dates as YYYY-MM-DD.\n",
            "Show dates as YYYY-MM-DD.\n"
            "Every tasks page heading shows the number of open tasks in brackets, for example 'Tasks (3)'.\n")
        rep(tasks, "- Calls: create_task\n", "- Calls: create_task, list_tasks\n")
    elif e == "E7":
        rep(tasks, "- Route: `/tasks`\n", "- Route: `/todo`\n")
    elif e == "E8":
        rep(about, "- explains: the page has the heading 'About' and links to tasks_index\n",
            "- explains: the page has the heading 'About this app' and links to tasks_index\n"
            "- trial_note: the page says 'Built for the sdlang trial.'\n")
    elif e == "E9":
        rep(tasks, "- required: the Add button is disabled while the title is empty\n",
            "- required: the Add button is disabled while the title is empty\n"
            "- too_long: a title over 80 characters shows 'Keep it under 80 characters' under the field and is not submitted\n")
    elif e == "E10":
        os.remove(about)
        rep(tasks, "- footer: the footer links to about\n", "")
    else:
        sys.exit(f"unknown edit {e}")


if __name__ == "__main__":
    edit(sys.argv[1], sys.argv[2])
