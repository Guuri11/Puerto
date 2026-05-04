use std::{fs, path::Path};

use crate::generators::naming::{apply, pascal_to_snake, to_pascal_case, write_file};
use crate::patchers::lib_rs::{patch_business_lib_domain_crud, patch_lib_block};

pub(crate) const MODEL: &str = r#"use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::errors::{Pascal}Error;

#[derive(Debug, Clone)]
pub struct {Pascal}Props {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct {Pascal} {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
}

impl {Pascal} {
    pub fn new(props: {Pascal}Props) -> Result<Self, {Pascal}Error> {
        if props.name.trim().is_empty() {
            return Err({Pascal}Error::ValidationError("name_empty".into()));
        }
        let now = chrono::Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            deleted: false,
            deleted_at: None,
            name: props.name,
        })
    }

    pub fn from_repository(data: {Pascal}) -> Self {
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_{snake}_when_name_is_valid() {
        let result = {Pascal}::new({Pascal}Props { name: "example".into() });
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "example");
    }

    #[test]
    fn should_reject_{snake}_when_name_is_empty() {
        let result = {Pascal}::new({Pascal}Props { name: "".into() });
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "{snake}.validation_error.name_empty"
        );
    }

    #[test]
    fn should_reject_{snake}_when_name_is_only_whitespace() {
        let result = {Pascal}::new({Pascal}Props { name: "   ".into() });
        assert!(result.is_err());
    }
}
"#;

pub(crate) const ERRORS: &str = r#"use thiserror::Error;

#[derive(Debug, Error)]
pub enum {Pascal}Error {
    #[error("{snake}.validation_error.{0}")]
    ValidationError(String),
    #[error("{snake}.not_found")]
    NotFound,
    #[error("{snake}.duplicate")]
    Duplicate,
    #[error("{snake}.repository_error")]
    RepositoryError,
    #[error("{snake}.unknown")]
    Unknown,
}
"#;

const CRUD_REPOSITORY: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use super::{errors::{Pascal}Error, model::{Pascal}};

#[async_trait]
pub trait {Pascal}RepositoryTrait: Send + Sync {
    async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use uuid::Uuid;

    use super::*;

    mock! {
        pub {Pascal}Repository {}

        #[async_trait]
        impl {Pascal}RepositoryTrait for {Pascal}Repository {
            async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error>;
            async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error>;
            async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error>;
        }
    }
}
"#;

pub(crate) const USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Create{Pascal}Params {
    pub name: String,
}

#[async_trait]
pub trait Create{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Create{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const GET_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Get{Pascal}Params {
    pub id: Uuid,
}

#[async_trait]
pub trait Get{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Get{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const LIST_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug)]
pub struct List{Pascal}Params;

#[async_trait]
pub trait List{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: List{Pascal}Params) -> Result<Vec<{Pascal}>, {Pascal}Error>;
}
"#;

const UPDATE_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone)]
pub struct Update{Pascal}Params {
    pub id: Uuid,
    pub name: String,
}

#[async_trait]
pub trait Update{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Update{Pascal}Params) -> Result<{Pascal}, {Pascal}Error>;
}
"#;

const DELETE_USE_CASE_TRAIT: &str = r#"use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{snake}::errors::{Pascal}Error;

#[derive(Debug, Clone)]
pub struct Delete{Pascal}Params {
    pub id: Uuid,
}

#[async_trait]
pub trait Delete{Pascal}UseCaseTrait: Send + Sync {
    async fn execute(&self, params: Delete{Pascal}Params) -> Result<(), {Pascal}Error>;
}
"#;

const OBJECT_MOTHER: &str = r#"use crate::domain::{snake}::model::{Pascal};
use crate::domain::{snake}::model::{Pascal}Props;

pub struct {Pascal}Mother {
    name: Option<String>,
}

impl {Pascal}Mother {
    pub fn new() -> Self {
        Self { name: None }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn with_empty_name(mut self) -> Self {
        self.name = Some(String::new());
        self
    }

    pub fn build(self) -> {Pascal} {
        {Pascal}::new({Pascal}Props {
            name: self.name.unwrap_or_else(|| "example".to_string()),
        })
        .expect("{Pascal}Mother: failed to build valid {Pascal}")
    }

    pub fn build_props(self) -> {Pascal}Props {
        {Pascal}Props {
            name: self.name.unwrap_or_else(|| "example".to_string()),
        }
    }

    pub fn random() -> {Pascal} {
        Self::new().build()
    }

    pub fn random_vec(n: usize) -> Vec<{Pascal}> {
        (0..n).map(|_| Self::random()).collect()
    }
}
"#;

pub fn write_domain_files(
    pascal: &str,
    snake: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("business/src/domain/{snake}/model.rs")),
        &apply(MODEL, pascal, snake),
    )?;
    write_file(
        &base.join(format!("business/src/domain/{snake}/errors.rs")),
        &apply(ERRORS, pascal, snake),
    )?;
    write_file(
        &base.join(format!("business/src/domain/{snake}/repository.rs")),
        &apply(CRUD_REPOSITORY, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/create_{snake}.rs"
        )),
        &apply(USE_CASE_TRAIT, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/get_{snake}.rs"
        )),
        &apply(GET_USE_CASE_TRAIT, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/list_{snake}.rs"
        )),
        &apply(LIST_USE_CASE_TRAIT, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/update_{snake}.rs"
        )),
        &apply(UPDATE_USE_CASE_TRAIT, pascal, snake),
    )?;
    write_file(
        &base.join(format!(
            "business/src/domain/{snake}/use_cases/delete_{snake}.rs"
        )),
        &apply(DELETE_USE_CASE_TRAIT, pascal, snake),
    )?;
    Ok(())
}

pub fn write_mother(pascal: &str, snake: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("business/src/tests/mothers/{snake}_mother.rs")),
        &apply(OBJECT_MOTHER, pascal, snake),
    )?;
    Ok(())
}

pub fn patch_mothers_lib(base: &Path, snake: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = base.join("business/src/lib.rs");
    let src = fs::read_to_string(&path)?;

    if src.contains(&format!("pub mod {snake}_mother;")) {
        return Ok(());
    }

    let new_mod = format!("\n        pub mod {snake}_mother;\n");

    if let Ok(patched) = patch_lib_block(&src, &["tests", "mothers"], &new_mod) {
        fs::write(&path, patched)?;
        return Ok(());
    }

    let mut content = src;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "\n#[cfg(test)]\npub mod tests {{\n    pub mod mothers {{\n        pub mod {snake}_mother;\n    }}\n}}\n"
    ));
    fs::write(&path, content)?;
    Ok(())
}

pub fn run_generate_domain(name: &str, base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} is already in puerto.toml. Use `puerto generate use-case` to add a use case."
        )
        .into());
    }

    write_domain_files(&pascal, &snake, base)?;
    write_mother(&pascal, &snake, base)?;
    patch_business_lib_domain_crud(base, &snake)?;
    patch_mothers_lib(base, &snake)?;

    let use_cases = vec![
        format!("create_{snake}"),
        format!("get_{snake}"),
        format!("list_{snake}"),
        format!("update_{snake}"),
        format!("delete_{snake}"),
    ];
    crate::puerto_toml::add_entity(base, &pascal, use_cases, config.project.db)?;

    println!("✓ business/domain/    — model, errors, repository trait, 5 use case traits");
    println!("✓ business/tests/     — {pascal}Mother (Object Mother)");
    println!("✓ puerto.toml         — {pascal} registered");
    println!();
    println!("  Next: puerto generate application {pascal}");
    Ok(())
}
