//! Accepting a proposal: the proposed libs' `.sd` files and the pending acquisitions
//! (`.spec/import.json`). A plan binds a commit, so the plan comes after these files are
//! committed: `sdc plan` reads the pending acquisitions and plans each as `create` with
//! strategy `acquire`, and publishing it removes it from the file.

use std::fs;
use std::path::Path;

use sdlang_plan::materialise::materialise;
use sdlang_plan::{pending_imports, write_pending_imports, Acquisition, PlanInputs};

use crate::error::{AdResult, AdoptError};
use crate::proposal::Proposal;

/// What acceptance wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Accepted {
    /// Spec files written, repository-relative.
    pub written: Vec<String>,
    /// Addresses now pending acquisition.
    pub pending: Vec<String>,
}

pub fn accept(root: &Path, p: &Proposal) -> AdResult<Accepted> {
    if !p.acceptable() {
        return Err(AdoptError::new(format!(
            "the proposal has {} collision(s); nothing was written",
            p.collisions.len()
        )));
    }
    if p.is_empty() {
        return Err(AdoptError::new("nothing to claim; nothing was written"));
    }
    for lib in &p.new_libs {
        if root.join(lib.file()).exists() {
            return Err(AdoptError::new(format!(
                "{} already exists; nothing was written",
                lib.file()
            )));
        }
    }
    let mut written: Vec<String> = Vec::new();
    let rollback = |written: &[String]| {
        for f in written {
            let _ = fs::remove_file(root.join(f));
        }
    };
    for lib in &p.new_libs {
        let file = lib.file();
        let full = root.join(&file);
        let result = fs::create_dir_all(full.parent().expect("under spec/"))
            .and_then(|_| fs::write(&full, lib.source()));
        if let Err(e) = result {
            rollback(&written);
            return Err(AdoptError::new(format!("{file}: {e}")));
        }
        written.push(file);
    }
    let check = || -> AdResult<std::collections::BTreeMap<String, Acquisition>> {
        let project = sdlang_graph::loader::load(root).map_err(|e| AdoptError::new(e.render()))?;
        let graph = sdlang_graph::lower::lower_project(&project)
            .map_err(|e| AdoptError::new(e.render()))?;
        let mats = materialise(&project, &graph)?;
        let mut pending = pending_imports(root)?;
        for (address, claim) in &p.claims {
            pending.insert(address.clone(), claim.acquisition());
        }
        for lib in &p.new_libs {
            let address = lib.address();
            let main = mats.get(&address).and_then(|m| m.slots.get("main"));
            if main != Some(&lib.path) {
                return Err(AdoptError::new(format!(
                    "the proposed lib {address} does not materialise at {}",
                    lib.path
                )));
            }
            let mut a = Acquisition::default();
            a.slots.insert("main".into());
            pending.insert(address, a);
        }
        // The acquisitions must plan: every address managed, new and with real slots.
        sdlang_plan::plan(&PlanInputs {
            root,
            project: &project,
            graph: &graph,
            plan_id: "import",
            baseline_commit: "git:import",
            acquire: Some(&pending),
            adopt: None,
        })?;
        Ok(pending)
    };
    let pending = match check() {
        Ok(p) => p,
        Err(e) => {
            rollback(&written);
            return Err(e);
        }
    };
    write_pending_imports(root, &pending)?;
    Ok(Accepted {
        written,
        pending: pending.keys().cloned().collect(),
    })
}
