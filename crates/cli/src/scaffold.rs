// Re-exports so existing call sites (`scaffold::run_scaffold`, etc.) continue to work.
pub use crate::generators::application::run_generate_application;
pub use crate::generators::bootstrap::run_bootstrap_command;
pub use crate::generators::domain::run_generate_domain;
pub use crate::generators::infrastructure::run_generate_repository;
pub use crate::generators::migration::run_generate_migration;
pub use crate::generators::presentation::run_generate_presentation;
pub use crate::generators::scaffold::{run_scaffold, run_value_object};
pub use crate::generators::use_case::run_use_case;
pub use crate::generators::web::run_generate_web;

// Used only in tests — keep visibility scoped to test builds.
#[cfg(test)]
pub use crate::generators::migration::run_migration;
#[cfg(test)]
pub use crate::generators::project::{apply_db_to_new_project, apply_no_demo, apply_no_web};
#[cfg(test)]
pub use crate::generators::scaffold::run;
