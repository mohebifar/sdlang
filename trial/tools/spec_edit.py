"""The language arm's spec change for one edit, as TRIAL.md's "The ten edits" states it.

Usage: python3 spec_edit.py <E1..E10> <arm dir>
Each edit is applied to the arm's spec/ tree after the previous ones; a text that is not
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
    s = lambda p: os.path.join(root, "spec", p)
    if e == "E1":
        rep(s("tasks/libs.int"), 'exports: ["insert", "all", "find", "save"]', 'exports: ["insert", "all", "find", "save", "remove"]')
        rep(s("tasks/libs.int"), '    unknown_id: "find returns null for an unknown id"\n',
            '    unknown_id: "find returns null for an unknown id"\n    removes: "after remove, find returns null and all omits the record"\n')
        with open(s("tasks/actions.int"), "a") as f:
            f.write('''
export action delete_task {
  summary: "Deletes a task"
  input: { id: string }
  uses: [store]
  errors {
    not_found: "No task has that id"
  }
}
''')
        rep(s("tasks/pages.int"), "calls: [list_tasks, complete_task]", "calls: [list_tasks, complete_task, delete_task]")
        rep(s("tasks/pages.int"), '    footer: "the footer links to @{about.about}"\n',
            '    footer: "the footer links to @{about.about}"\n    delete: "each row has a Delete button that removes the task from the list"\n')
    elif e == "E2":
        rep(s("tasks/actions.int"), "export action create_task {", 'type TaskStatus = "open" | "done"\n\nexport action create_task {')
        rep(s("tasks/actions.int"), '''  input: { }
  output: [{ id: string, title: string, done: boolean, created_at: timestamp }]
  uses: [store]
  expect {
    order: "open tasks come before done ones; within each group, newest first"
  }''', '''  input: { status: TaskStatus? }
  output: [{ id: string, title: string, done: boolean, created_at: timestamp }]
  uses: [store]
  expect {
    order: "open tasks come before done ones; within each group, newest first"
    filter: "with a status, only tasks with that status are returned; without one, all"
  }''')
    elif e == "E3":
        rep(s("tasks/actions.int"), '''export action complete_task {
  summary: "Marks a task done"
  input: { id: string }
  output: { id: string, done: boolean }
  uses: [store]
  errors {
    not_found: "No task has that id"
  }
  expect {
    marks_done: "after complete_task, the stored task is done"
  }
}''', '''moved tasks.complete_task -> tasks.set_task_done

export action set_task_done {
  summary: "Marks a task done or open"
  input: { id: string, done: boolean }
  output: { id: string, done: boolean }
  uses: [store]
  errors {
    not_found: "No task has that id"
  }
  expect {
    toggles: "set_task_done with done false makes a done task open again"
  }
}''')
        rep(s("tasks/pages.int"), "One row per task: title, created date and a Complete button for open tasks.",
            "One row per task: title, created date and a checkbox that toggles done.")
        rep(s("tasks/pages.int"), "calls: [list_tasks, complete_task, delete_task]", "calls: [list_tasks, set_task_done, delete_task]")
        rep(s("tasks/pages.int"), '    complete: "Complete marks the task done and moves it below the open tasks"\n',
            '    toggle: "the checkbox toggles a task between open and done, moving it between the groups"\n')
    elif e == "E4":
        rep(s("tasks/actions.int"), '''  summary: "Adds a task"
''', '''  summary: "Adds a task"
  prompt: """Store the title with surrounding whitespace removed and inner runs of spaces collapsed to one."""
''')
    elif e == "E5":
        rep(s("tasks/actions.int"), "  output: [{ id: string, title: string, done: boolean, created_at: timestamp }]",
            "  output: [{ id: string, title: string, done: boolean, created_at: timestamp, done_at: timestamp? }]")
        rep(s("tasks/actions.int"), '    filter: "with a status, only tasks with that status are returned; without one, all"\n',
            '    filter: "with a status, only tasks with that status are returned; without one, all"\n'
            '    done_at: "done_at is set when a task is marked done and null when it is open"\n')
        rep(s("tasks/actions.int"), "  input: { id: string, done: boolean }\n  output: { id: string, done: boolean }",
            "  input: { id: string, done: boolean }\n  output: { id: string, done: boolean, done_at: timestamp? }")
        rep(s("tasks/actions.int"), '    toggles: "set_task_done with done false makes a done task open again"\n',
            '    toggles: "set_task_done with done false makes a done task open again"\n'
            '    done_at: "done_at is set when a task is marked done and null when it is open"\n')
    elif e == "E6":
        rep(s("tasks/module.int"), "    Show dates as YYYY-MM-DD.\n",
            "    Show dates as YYYY-MM-DD.\n    Every tasks page heading shows the number of open tasks in brackets, for example 'Tasks (3)'.\n")
        rep(s("tasks/pages.int"), "  calls: [create_task]", "  calls: [create_task, list_tasks]")
    elif e == "E7":
        rep(s("tasks/pages.int"), 'route: "/tasks"\n', 'route: "/todo"\n')
    elif e == "E8":
        rep(s("about/about.int"), "explains: \"the page has the heading 'About' and links to @{tasks.tasks_index}\"",
            "explains: \"the page has the heading 'About this app' and links to @{tasks.tasks_index}\"\n"
            "    trial_note: \"the page says 'Built for the int lang trial.'\"")
    elif e == "E9":
        rep(s("tasks/pages.int"), '    required: "the Add button is disabled while the title is empty"\n',
            '    required: "the Add button is disabled while the title is empty"\n'
            "    too_long: \"a title over 80 characters shows 'Keep it under 80 characters' under the field and is not submitted\"\n")
    elif e == "E10":
        os.remove(s("about/about.int"))
        os.rmdir(s("about"))
        rep(s("tasks/pages.int"), "import about\n", "")
        rep(s("tasks/pages.int"), '    footer: "the footer links to @{about.about}"\n', "")
    else:
        sys.exit(f"unknown edit {e}")


if __name__ == "__main__":
    edit(sys.argv[1], sys.argv[2])
