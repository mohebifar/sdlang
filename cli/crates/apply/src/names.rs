//! Lenient names in a result (docs/protocol.md, Result): an agent may name a requirement or an
//! output by its full member or by any unambiguous tail of it, so `filter`,
//! `expect.filter` and `tasks.list_tasks#expect.filter` all name the same requirement.

/// Resolves `name` against full `members` such as `tasks.list_tasks#expect.filter`.
pub fn resolve<'a>(
    name: &str,
    members: impl IntoIterator<Item = &'a str>,
) -> Result<String, String> {
    let all: Vec<&str> = members.into_iter().collect();
    if all.contains(&name) {
        return Ok(name.to_string());
    }
    let name = name.trim_start_matches('#');
    let hits: Vec<&str> = all
        .iter()
        .copied()
        .filter(|m| {
            let tail = m.split_once('#').map_or(*m, |(_, t)| t);
            tail == name
                || tail.ends_with(&format!(".{name}"))
                || m.ends_with(&format!(".{name}"))
                || m.ends_with(&format!("#{name}"))
        })
        .collect();
    match hits.as_slice() {
        [one] => Ok(one.to_string()),
        [] => Err(format!("{name:?} names none of: {}", all.join(", "))),
        many => Err(format!("{name:?} could be any of: {}", many.join(", "))),
    }
}

/// The key a member ends with: `filter` for `tasks.list_tasks#expect.filter`. A test case
/// tagged with a requirement's key covers that requirement unless a result says otherwise.
pub fn key(member: &str) -> &str {
    member.rsplit(['.', '#']).next().unwrap_or(member)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQS: [&str; 3] = [
        "tasks.list_tasks#expect.filter",
        "tasks.list_tasks#expect.order",
        "tasks.delete_task#errors.not_found",
    ];

    #[test]
    fn full_and_short_names_resolve() {
        for name in [
            "tasks.list_tasks#expect.filter",
            "expect.filter",
            "filter",
            "list_tasks#expect.filter",
        ] {
            assert_eq!(resolve(name, REQS).unwrap(), REQS[0], "{name}");
        }
        assert_eq!(resolve("not_found", REQS).unwrap(), REQS[2]);
    }

    #[test]
    fn unknown_and_ambiguous_names_are_errors() {
        assert!(resolve("missing", REQS)
            .unwrap_err()
            .contains("names none of"));
        let outputs = ["a.x#outputs.tests", "a.y#outputs.tests"];
        assert!(resolve("tests", outputs)
            .unwrap_err()
            .contains("could be any of"));
        assert_eq!(resolve("x#outputs.tests", outputs).unwrap(), outputs[0]);
    }

    #[test]
    fn key_is_the_last_segment() {
        assert_eq!(key(REQS[0]), "filter");
        assert_eq!(key(REQS[2]), "not_found");
    }
}
