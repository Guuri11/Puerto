use crate::commands::list::{require_puerto_project, run_list};
use crate::commands::new::extract_template;
use crate::commands::validate;
use crate::generators::conflict::OverwritePolicy;
use crate::puerto_toml::ValueObjectDefinition;
use crate::{puerto_toml, scaffold, snippets};
use cargo_generate::{GenerateArgs, TemplatePath, Vcs, generate};
use std::fs;
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("puerto_test_{name}"))
}

fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn generate_project(name: &str, destination: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let tmp = extract_template()?;

    let args = GenerateArgs {
        template_path: TemplatePath {
            path: Some(tmp.path().to_string_lossy().into_owned()),
            ..Default::default()
        },
        name: Some(name.to_string()),
        destination: Some(destination.to_path_buf()),
        vcs: Some(Vcs::None),
        no_workspace: true,
        ..Default::default()
    };

    let output_dir = generate(args)?;
    drop(tmp);
    Ok(output_dir)
}

mod auth;
mod bootstrap_gen;
mod cli_surface;
mod compile_matrix;
mod conventions;
mod domain_events;
mod field_args;
mod generated_output;
mod ide_snippets;
mod inmemory_repository;
mod layered_generators;
mod mail;
mod mail_e2e;
mod manifest_validation;
mod migrations;
mod new_project;
mod paged_lists;
mod patchers;
mod repository_errors;
mod runtime_crates;
mod scaffolding;
mod startup_errors;
mod typed_fields;
mod typed_ids;
mod use_cases;
mod value_objects;
mod walkthrough_fixes;
mod web_slice;
mod worker;
mod worker_e2e;

// Helpers shared across the test modules. Each lives next to the tests that introduced it.
use compile_matrix::{assert_generated_project_is_clean, scenario_workspace, test_database_server};
use new_project::new_project_non_interactive;
use scaffolding::{scaffold_test_field, setup_puerto_stubs, setup_puerto_stubs_with_project_db};
use use_cases::setup_use_case_stubs;
