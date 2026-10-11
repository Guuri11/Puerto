use super::*;
use compile_matrix::{
    CUSTOM_USE_CASE, ENUM_ONLY, ENUM_VO, MULTI_ENTITY, NO_DEMO, NO_WEB, OPT_VEC_PRIM, OPT_VEC_VO,
    PLAIN_VO, POSTGRES_EVERY_TYPE, PRIMITIVES, SHARED_VO, Scenario, UUID_VO, VO_KITCHEN_SINK,
    apply_scenario,
};
use std::collections::BTreeMap;

// ── T-18: snapshots of the generated output ───────────────────────────────
//
// The compile matrix proves the generated code builds; it cannot see a change that still builds.
// These snapshots lock the exact output of every matrix scenario: each one is every file the
// generators create, rewrite or delete relative to the raw template — the template's static files
// are not Puerto's generation logic and would only bury the signal.
//
// After an intended change to generated output: `INSTA_UPDATE=always make test`, then read the
// `.snap` diff in full before committing — it is exactly what lands in users' projects.

fn project_tree(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let content = String::from_utf8_lossy(&fs::read(&path).unwrap()).into_owned();
                out.insert(rel, content);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Migration versions are timestamps; everything else Puerto writes is deterministic.
fn normalize_path(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, file)) if dir.ends_with("migrations") => match file.split_once('_') {
            Some((version, rest)) if version.chars().all(|c| c.is_ascii_digit()) => {
                format!("{dir}/<version>_{rest}")
            }
            _ => path.to_string(),
        },
        _ => path.to_string(),
    }
}

/// Everything the scenario changed in a freshly generated project, as one reviewable document.
fn generated_output(scenario: &Scenario) -> String {
    let dir = temp_dir(&format!("snapshot_{}", scenario.name));
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("snapshot", &dir).unwrap();

    let before = project_tree(&output);
    apply_scenario(scenario, &output);
    let after = project_tree(&output);
    cleanup(&dir);

    let mut sections = BTreeMap::new();
    for (path, content) in &after {
        if before.get(path) != Some(content) {
            sections.insert(normalize_path(path), content.clone());
        }
    }
    for path in before.keys().filter(|p| !after.contains_key(*p)) {
        sections.insert(normalize_path(path), "(deleted)\n".to_string());
    }
    sections
        .iter()
        .map(|(path, content)| format!("=== {path} ===\n{content}"))
        .collect::<Vec<_>>()
        .join("\n")
}

macro_rules! snapshot_scenarios {
    ($($test:ident => $scenario:ident),* $(,)?) => {$(
        #[test]
        fn $test() {
            insta::assert_snapshot!($scenario.name, generated_output(&$scenario));
        }
    )*};
}

snapshot_scenarios! {
    snapshot_no_demo => NO_DEMO,
    snapshot_no_web => NO_WEB,
    snapshot_primitives => PRIMITIVES,
    snapshot_uuid_vo => UUID_VO,
    snapshot_opt_vec_prim => OPT_VEC_PRIM,
    snapshot_plain_vo => PLAIN_VO,
    snapshot_opt_vec_vo => OPT_VEC_VO,
    snapshot_enum_vo => ENUM_VO,
    snapshot_shared_vo => SHARED_VO,
    snapshot_multi_entity => MULTI_ENTITY,
    snapshot_vo_kitchen_sink => VO_KITCHEN_SINK,
    snapshot_enum_only => ENUM_ONLY,
    snapshot_custom_use_case => CUSTOM_USE_CASE,
    snapshot_postgres_every_type => POSTGRES_EVERY_TYPE,
}

#[test]
fn should_hide_only_the_migration_version_from_snapshots() {
    assert_eq!(
        normalize_path("infrastructure/migrations/20261011093000_create_product_table.sql"),
        "infrastructure/migrations/<version>_create_product_table.sql"
    );
    assert_eq!(
        normalize_path("business/src/domain/product/model.rs"),
        "business/src/domain/product/model.rs"
    );
}
