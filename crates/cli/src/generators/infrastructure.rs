use std::path::Path;

use crate::generators::migration::run_migration;
use crate::generators::naming::{apply, pascal_to_snake, to_pascal_case, write_file};
use crate::generators::types::{
    is_enum_vo, is_option_vo, is_shared_vo, is_value_object, is_vec_vo, resolve_type, vo_inner_type,
};
use crate::patchers::lib_rs::patch_infra_lib;
use crate::puerto_toml::{Field, ValueObjectDefinition};

pub(crate) const INFRA_REPOSITORY: &str = r#"use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

pub struct InMemory{Pascal}Repository {
    logger: Arc<dyn LoggerTrait>,
}

impl InMemory{Pascal}Repository {
    pub fn new(logger: Arc<dyn LoggerTrait>) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for InMemory{Pascal}Repository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        Ok(None)
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        Ok(())
    }
}
"#;

pub(crate) const INFRA_DB_REPOSITORY: &str = r#"use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

use super::entity::{Pascal}Db;

pub struct Pg{Pascal}Repository {
    pub pool: PgPool,
    logger: Arc<dyn LoggerTrait>,
}

impl Pg{Pascal}Repository {
    pub fn new(pool: PgPool, logger: Arc<dyn LoggerTrait>) -> Self {
        Self { pool, logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for Pg{Pascal}Repository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        let row = sqlx::query_as!(
            {Pascal}Db,
            "SELECT id, created_at, updated_at, deleted, deleted_at, name
             FROM {snake}s WHERE id = $1 AND deleted = false",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("find_by_id error: {e}"));
            {Pascal}Error::RepositoryError
        })?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        let db = {Pascal}Db::from(entity);
        sqlx::query!(
            "INSERT INTO {snake}s (id, created_at, updated_at, deleted, deleted_at, name)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO UPDATE
             SET updated_at = $3, deleted = $4, deleted_at = $5, name = $6",
            db.id,
            db.created_at,
            db.updated_at,
            db.deleted,
            db.deleted_at,
            db.name
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error() {
                if db_err.code().map_or(false, |c| c == "23505") {
                    return {Pascal}Error::Duplicate;
                }
            }
            self.logger.error(&format!("save error: {e}"));
            {Pascal}Error::RepositoryError
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use business::domain::{snake}::model::{Pascal}Props;
    use std::sync::Arc;

    struct TestLogger;
    impl business::domain::logger::LoggerTrait for TestLogger {
        fn info(&self, _: &str) {}
        fn warn(&self, _: &str) {}
        fn error(&self, _: &str) {}
        fn debug(&self, _: &str) {}
    }

    fn test_repo(pool: PgPool) -> Pg{Pascal}Repository {
        Pg{Pascal}Repository::new(pool, Arc::new(TestLogger))
    }

    async fn seed(pool: &PgPool, name: &str) -> {Pascal} {
        let entity = {Pascal}::new({Pascal}Props { name: name.to_string() }).unwrap();
        test_repo(pool.clone()).save(&entity).await.unwrap();
        entity
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn should_persist_and_retrieve_by_id(pool: PgPool) {
        // Arrange
        let entity = seed(&pool, "example").await;

        // Act
        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();

        // Assert
        assert_eq!(found.id, entity.id);
        assert_eq!(found.name, entity.name);
        assert!(!found.deleted);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn should_return_none_for_nonexistent_id(pool: PgPool) {
        // Act
        let result = test_repo(pool).find_by_id(Uuid::new_v4()).await.unwrap();

        // Assert
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn should_update_entity_on_save_conflict(pool: PgPool) {
        // Arrange
        let mut entity = seed(&pool, "original").await;
        entity.name = "updated".to_string();
        entity.updated_at = chrono::Utc::now();

        // Act
        test_repo(pool.clone()).save(&entity).await.unwrap();

        // Assert
        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();
        assert_eq!(found.name, "updated");
    }
}
"#;

pub(crate) const INFRA_ENTITY: &str = r#"use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use business::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct {Pascal}Db {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
}

impl TryFrom<{Pascal}Db> for {Pascal} {
    type Error = {Pascal}Error;

    fn try_from(row: {Pascal}Db) -> Result<Self, Self::Error> {
        Ok(Self::from_repository({Pascal} {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted: row.deleted,
            deleted_at: row.deleted_at,
            name: row.name,
        }))
    }
}

impl From<&{Pascal}> for {Pascal}Db {
    fn from(entity: &{Pascal}) -> Self {
        Self {
            id: entity.id,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            deleted: entity.deleted,
            deleted_at: entity.deleted_at,
            name: entity.name.clone(),
        }
    }
}
"#;

const CRUD_INFRA_REPOSITORY: &str = r#"use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

pub struct InMemory{Pascal}Repository {
    logger: Arc<dyn LoggerTrait>,
}

impl InMemory{Pascal}Repository {
    pub fn new(logger: Arc<dyn LoggerTrait>) -> Self {
        Self { logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for InMemory{Pascal}Repository {
    async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error> {
        self.logger.debug("find_all");
        Ok(vec![])
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        Ok(None)
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        Ok(())
    }
}
"#;

pub const DB_RS: &str = r#"use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database.connection_error")]
    ConnectionError,
    #[error("database.migration_error")]
    MigrationError,
}

pub async fn create_postgres_pool(database_url: &str) -> Result<PgPool, DatabaseError> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .connect(database_url)
        .await
        .map_err(|_| DatabaseError::ConnectionError)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), DatabaseError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|_| DatabaseError::MigrationError)
}
"#;

// ── Dynamic generators ────────────────────────────────────────────────────────

fn effective_fields(fields: &[Field]) -> Vec<Field> {
    if fields.is_empty() {
        vec![Field {
            name: "name".into(),
            field_type: "String".into(),
            unique: false,
            value_object: None,
            value_object_kind: None,
            enum_variants: None,
        }]
    } else {
        fields.to_vec()
    }
}

fn field_needs_clone(field_type: &str) -> bool {
    matches!(
        field_type,
        "String" | "Option<String>" | "Vec<String>" | "Vec<i64>" | "HashMap<String, String>"
    )
}

fn sql_ddl_col(name: &str, field_type: &str) -> String {
    let mapping = resolve_type(field_type).unwrap();
    let sql_base = match mapping.sql_type {
        "DOUBLE" => "DOUBLE PRECISION",
        other => other,
    };
    if mapping.sql_nullable {
        format!("    {name} {sql_base}")
    } else {
        let suffix = match mapping.sql_type {
            "TEXT[]" | "BIGINT[]" => " NOT NULL DEFAULT '{}'",
            "JSONB" => " NOT NULL DEFAULT '{}'",
            _ => " NOT NULL",
        };
        format!("    {name} {sql_base}{suffix}")
    }
}

fn sql_col_list(eff: &[Field]) -> String {
    let custom: String = eff.iter().map(|f| format!(", {}", f.name)).collect();
    format!("id, created_at, updated_at, deleted, deleted_at{custom}")
}

fn sql_params_list(n: usize) -> String {
    let custom: String = (6..=n).map(|i| format!(", ${i}")).collect();
    format!("$1, $2, $3, $4, $5{custom}")
}

fn sql_conflict_set(eff: &[Field]) -> String {
    let custom: String = eff
        .iter()
        .enumerate()
        .map(|(i, f)| format!(", {} = ${}", f.name, 6 + i))
        .collect();
    format!("updated_at = $3, deleted = $4, deleted_at = $5{custom}")
}

fn db_bindings_str(eff: &[Field]) -> String {
    let mut lines = vec![
        "            db.id,".to_string(),
        "            db.created_at,".to_string(),
        "            db.updated_at,".to_string(),
        "            db.deleted,".to_string(),
        "            db.deleted_at,".to_string(),
    ];
    for f in eff {
        // SQLx requires array fields to be passed as slice references (&[T])
        let binding = if f.field_type.starts_with("Vec<") {
            format!("            &db.{},", f.name)
        } else {
            format!("            db.{},", f.name)
        };
        lines.push(binding);
    }
    lines.join("\n")
}

fn seed_fn_str(pascal: &str, eff: &[Field], shared_vos: &[crate::puerto_toml::ValueObjectDefinition]) -> String {
    let props_lines: String = eff
        .iter()
        .map(|f| {
            if is_option_vo(f) {
                format!("            {}: None,", f.name)
            } else if is_vec_vo(f) {
                format!("            {}: vec![],", f.name)
            } else if is_enum_vo(f) {
                let vo = f.value_object.as_deref().unwrap();
                let first_variant = f.enum_variants.as_deref().unwrap().first().unwrap();
                format!("            {}: {}::{},", f.name, vo, first_variant)
            } else if is_value_object(f) {
                let mapping = resolve_type(&f.field_type).unwrap();
                let vo = f.value_object.as_deref().unwrap();
                format!(
                    "            {}: {}::new({}).expect(\"valid {}\"),",
                    f.name, vo, mapping.default_expr, vo
                )
            } else {
                let mapping = resolve_type(&f.field_type).unwrap();
                format!("            {}: {},", f.name, mapping.default_expr)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = shared_vos; // used by callers for import generation
    format!(
        "    async fn seed(pool: &PgPool) -> {pascal} {{\n        let entity = {pascal}::new({pascal}Props {{\n{props_lines}\n        }}).unwrap();\n        test_repo(pool.clone()).save(&entity).await.unwrap();\n        entity\n    }}\n",
        pascal = pascal,
        props_lines = props_lines,
    )
}

fn field_asserts_str(eff: &[Field]) -> String {
    eff.iter()
        .map(|f| format!("        assert_eq!(found.{}, entity.{});", f.name, f.name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn update_test_str(eff: &[Field]) -> String {
    // Prefer a primitive String field; fall back to a VO String field
    let sf = eff
        .iter()
        .find(|f| f.field_type == "String" && !is_value_object(f))
        .or_else(|| {
            eff.iter()
                .find(|f| f.field_type == "String" && is_value_object(f) && !is_enum_vo(f) && !is_option_vo(f) && !is_vec_vo(f))
        });
    if let Some(sf) = sf {
        if is_value_object(sf) {
            let vo = sf.value_object.as_deref().unwrap();
            format!(
                "        let mut entity = seed(&pool).await;\n        entity.{field} = {vo}::new(\"updated\".to_string()).unwrap();\n        entity.updated_at = chrono::Utc::now();\n\n        // Act\n        test_repo(pool.clone()).save(&entity).await.unwrap();\n\n        // Assert\n        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();\n        assert_eq!(found.{field}.value(), \"updated\");",
                field = sf.name,
                vo = vo,
            )
        } else {
            format!(
                "        let mut entity = seed(&pool).await;\n        entity.{field} = \"updated\".to_string();\n        entity.updated_at = chrono::Utc::now();\n\n        // Act\n        test_repo(pool.clone()).save(&entity).await.unwrap();\n\n        // Assert\n        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();\n        assert_eq!(found.{field}, \"updated\");",
                field = sf.name,
            )
        }
    } else {
        "        let entity = seed(&pool).await;\n\n        // Act\n        test_repo(pool.clone()).save(&entity).await.unwrap();\n\n        // Assert\n        let found = test_repo(pool).find_by_id(entity.id).await.unwrap();\n        assert!(found.is_some());".to_string()
    }
}

pub fn generate_infra_entity(
    pascal: &str,
    snake: &str,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> String {
    let eff = effective_fields(fields);

    let struct_fields: String = eff
        .iter()
        .map(|f| format!("    pub {}: {},", f.name, f.field_type))
        .collect::<Vec<_>>()
        .join("\n");

    let mut vo_imports: Vec<String> = vec![];
    let has_vo = eff.iter().any(is_value_object);
    if has_vo {
        for f in eff.iter().filter(|f| is_value_object(f)) {
            let vo = f.value_object.as_deref().unwrap();
            let stmt = if is_shared_vo(f, shared_vos) {
                format!("use business::domain::shared::value_objects::{};", vo)
            } else {
                format!("use business::domain::{}::value_objects::{};", snake, vo)
            };
            if !vo_imports.contains(&stmt) {
                vo_imports.push(stmt);
            }
        }
    }
    let vo_imports_str = if vo_imports.is_empty() {
        String::new()
    } else {
        format!("\n{}", vo_imports.join("\n"))
    };

    let try_from_fields: String = eff
        .iter()
        .map(|f| {
            if is_enum_vo(f) {
                let vo = f.value_object.as_deref().unwrap();
                if is_shared_vo(f, shared_vos) {
                    format!("            {}: {}::from_str(&row.{}).map_err(|_| {}Error::Invalid{})?,", f.name, vo, f.name, pascal, vo)
                } else {
                    format!("            {}: {}::from_str(&row.{})?,", f.name, vo, f.name)
                }
            } else if is_option_vo(f) {
                let vo = f.value_object.as_deref().unwrap();
                if is_shared_vo(f, shared_vos) {
                    format!("            {}: row.{}.map({vo}::new).transpose().map_err(|_| {}Error::Invalid{})?,", f.name, f.name, pascal, vo, vo = vo)
                } else {
                    format!("            {}: row.{}.map({vo}::new).transpose()?,", f.name, f.name, vo = vo)
                }
            } else if is_vec_vo(f) {
                let vo = f.value_object.as_deref().unwrap();
                if is_shared_vo(f, shared_vos) {
                    format!("            {}: row.{}.into_iter().map({vo}::new).collect::<Result<Vec<_>,_>>().map_err(|_| {}Error::Invalid{})?,", f.name, f.name, pascal, vo, vo = vo)
                } else {
                    format!("            {}: row.{}.into_iter().map({vo}::new).collect::<Result<Vec<_>,_>>()?,", f.name, f.name, vo = vo)
                }
            } else if is_value_object(f) {
                let vo = f.value_object.as_deref().unwrap();
                if is_shared_vo(f, shared_vos) {
                    format!("            {}: {}::new(row.{}).map_err(|_| {}Error::Invalid{})?,", f.name, vo, f.name, pascal, vo)
                } else {
                    format!("            {}: {}::new(row.{})?,", f.name, vo, f.name)
                }
            } else {
                format!("            {}: row.{},", f.name, f.name)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let from_fields: String = eff
        .iter()
        .map(|f| {
            if is_enum_vo(f) {
                format!("            {}: entity.{}.as_str().to_string(),", f.name, f.name)
            } else if is_option_vo(f) {
                let inner = vo_inner_type(f);
                if inner == "String" {
                    format!("            {}: entity.{}.as_ref().map(|v| v.value().to_string()),", f.name, f.name)
                } else {
                    format!("            {}: entity.{}.map(|v| v.value()),", f.name, f.name)
                }
            } else if is_vec_vo(f) {
                let inner = vo_inner_type(f);
                if inner == "String" {
                    format!("            {}: entity.{}.iter().map(|v| v.value().to_string()).collect(),", f.name, f.name)
                } else {
                    format!("            {}: entity.{}.iter().map(|v| v.value()).collect(),", f.name, f.name)
                }
            } else if is_value_object(f) {
                match f.field_type.as_str() {
                    "String" => format!(
                        "            {}: entity.{}.value().to_string(),",
                        f.name, f.name
                    ),
                    _ => format!("            {}: entity.{}.value(),", f.name, f.name),
                }
            } else if field_needs_clone(&f.field_type) {
                format!("            {}: entity.{}.clone(),", f.name, f.name)
            } else {
                format!("            {}: entity.{},", f.name, f.name)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let template = r#"use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;{vo_imports}

use business::domain::{snake}::{errors::{Pascal}Error, model::{Pascal}};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct {Pascal}Db {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
{struct_fields}
}

impl TryFrom<{Pascal}Db> for {Pascal} {
    type Error = {Pascal}Error;

    fn try_from(row: {Pascal}Db) -> Result<Self, Self::Error> {
        Ok(Self::from_repository({Pascal} {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted: row.deleted,
            deleted_at: row.deleted_at,
{try_from_fields}
        }))
    }
}

impl From<&{Pascal}> for {Pascal}Db {
    fn from(entity: &{Pascal}) -> Self {
        Self {
            id: entity.id,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            deleted: entity.deleted,
            deleted_at: entity.deleted_at,
{from_fields}
        }
    }
}
"#;

    template
        .replace("{Pascal}", pascal)
        .replace("{snake}", snake)
        .replace("{struct_fields}", &struct_fields)
        .replace("{try_from_fields}", &try_from_fields)
        .replace("{from_fields}", &from_fields)
        .replace("{vo_imports}", &vo_imports_str)
}

pub fn generate_crud_infra_db_repository(pascal: &str, snake: &str, fields: &[Field], shared_vos: &[crate::puerto_toml::ValueObjectDefinition]) -> String {
    let eff = effective_fields(fields);
    let n = 5 + eff.len();

    let all_cols = sql_col_list(&eff);
    let all_params = sql_params_list(n);
    let all_updates = sql_conflict_set(&eff);
    let all_bindings = db_bindings_str(&eff);
    let seed_fn = seed_fn_str(pascal, &eff, shared_vos);
    let field_asserts = field_asserts_str(&eff);
    let update_test = update_test_str(&eff);

    // VO imports needed in the integration test module (for seed/update helpers)
    let mut vo_test_imports_vec: Vec<String> = vec![];
    for f in eff.iter().filter(|f| is_value_object(f) && !is_option_vo(f) && !is_vec_vo(f) && !is_enum_vo(f)) {
        let vo = f.value_object.as_deref().unwrap();
        let stmt = if is_shared_vo(f, shared_vos) {
            format!("    use business::domain::shared::value_objects::{};", vo)
        } else {
            format!("    use business::domain::{}::value_objects::{};", snake, vo)
        };
        if !vo_test_imports_vec.contains(&stmt) {
            vo_test_imports_vec.push(stmt);
        }
    }
    let vo_test_imports = if vo_test_imports_vec.is_empty() {
        String::new()
    } else {
        format!("\n{}", vo_test_imports_vec.join("\n"))
    };

    let template = r#"use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use business::domain::{snake}::{
    errors::{Pascal}Error,
    model::{Pascal},
    repository::{Pascal}RepositoryTrait,
};
use business::domain::logger::LoggerTrait;

use super::entity::{Pascal}Db;

pub struct Pg{Pascal}Repository {
    pub pool: PgPool,
    logger: Arc<dyn LoggerTrait>,
}

impl Pg{Pascal}Repository {
    pub fn new(pool: PgPool, logger: Arc<dyn LoggerTrait>) -> Self {
        Self { pool, logger }
    }
}

#[async_trait]
impl {Pascal}RepositoryTrait for Pg{Pascal}Repository {
    async fn find_all(&self) -> Result<Vec<{Pascal}>, {Pascal}Error> {
        self.logger.debug("find_all");
        let rows = sqlx::query_as!(
            {Pascal}Db,
            "SELECT {all_cols}
             FROM {snake}s WHERE deleted = false ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("find_all error: {e}"));
            {Pascal}Error::RepositoryError
        })?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<{Pascal}>, {Pascal}Error> {
        self.logger.debug(&format!("find_by_id: {id}"));
        let row = sqlx::query_as!(
            {Pascal}Db,
            "SELECT {all_cols}
             FROM {snake}s WHERE id = $1 AND deleted = false",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            self.logger.error(&format!("find_by_id error: {e}"));
            {Pascal}Error::RepositoryError
        })?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn save(&self, entity: &{Pascal}) -> Result<(), {Pascal}Error> {
        self.logger.debug(&format!("save: {}", entity.id));
        let db = {Pascal}Db::from(entity);
        sqlx::query!(
            "INSERT INTO {snake}s ({all_cols})
             VALUES ({all_params})
             ON CONFLICT (id) DO UPDATE
             SET {all_updates}",
{all_bindings}
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error() {
                if db_err.code().map_or(false, |c| c == "23505") {
                    return {Pascal}Error::Duplicate;
                }
            }
            self.logger.error(&format!("save error: {e}"));
            {Pascal}Error::RepositoryError
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use business::domain::{snake}::model::{Pascal}Props;
    use std::sync::Arc;
{vo_test_imports}
    struct TestLogger;
    impl business::domain::logger::LoggerTrait for TestLogger {
        fn info(&self, _: &str) {}
        fn warn(&self, _: &str) {}
        fn error(&self, _: &str) {}
        fn debug(&self, _: &str) {}
    }

    fn test_repo(pool: PgPool) -> Pg{Pascal}Repository {
        Pg{Pascal}Repository::new(pool, Arc::new(TestLogger))
    }

{seed_fn}

    #[sqlx::test(migrations = "./migrations")]
    async fn should_persist_and_retrieve_by_id(pool: PgPool) {
        // Arrange
        let entity = seed(&pool).await;

        // Act
        let found = test_repo(pool).find_by_id(entity.id).await.unwrap().unwrap();

        // Assert
        assert_eq!(found.id, entity.id);
{field_asserts}
        assert!(!found.deleted);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn should_return_none_for_nonexistent_id(pool: PgPool) {
        // Act
        let result = test_repo(pool).find_by_id(Uuid::new_v4()).await.unwrap();

        // Assert
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn should_update_entity_on_save_conflict(pool: PgPool) {
        // Arrange
{update_test}
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn should_list_all_{snake}s_excluding_deleted(pool: PgPool) {
        // Arrange
        seed(&pool).await;
        seed(&pool).await;

        // Act
        let results = test_repo(pool).find_all().await.unwrap();

        // Assert
        assert_eq!(results.len(), 2);
    }
}
"#;

    template
        .replace("{Pascal}", pascal)
        .replace("{snake}", snake)
        .replace("{all_cols}", &all_cols)
        .replace("{all_params}", &all_params)
        .replace("{all_updates}", &all_updates)
        .replace("{all_bindings}", &all_bindings)
        .replace("{seed_fn}", &seed_fn)
        .replace("{field_asserts}", &field_asserts)
        .replace("{update_test}", &update_test)
        .replace("{vo_test_imports}", &vo_test_imports)
}

pub fn create_table_sql(snake: &str, fields: &[Field]) -> String {
    let eff = effective_fields(fields);
    let custom_cols: String = eff
        .iter()
        .map(|f| format!("{},", sql_ddl_col(&f.name, &f.field_type)))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "CREATE TABLE {snake}s (\n    id UUID PRIMARY KEY,\n{custom_cols}\n    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    deleted BOOLEAN NOT NULL DEFAULT FALSE,\n    deleted_at TIMESTAMPTZ\n);\n",
        snake = snake,
        custom_cols = custom_cols,
    )
}

pub fn write_repository_files(
    pascal: &str,
    snake: &str,
    base: &Path,
    db: bool,
    fields: &[Field],
    shared_vos: &[ValueObjectDefinition],
) -> Result<(), Box<dyn std::error::Error>> {
    if db {
        write_file(
            &base.join(format!("infrastructure/src/{snake}/entity.rs")),
            &generate_infra_entity(pascal, snake, fields, shared_vos),
        )?;
        write_file(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &generate_crud_infra_db_repository(pascal, snake, fields, shared_vos),
        )?;
    } else {
        write_file(
            &base.join(format!("infrastructure/src/{snake}/repository.rs")),
            &apply(CRUD_INFRA_REPOSITORY, pascal, snake),
        )?;
    }
    Ok(())
}

pub fn run_generate_repository(
    name: &str,
    base: &Path,
    sqlx_bin: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = crate::puerto_toml::read(base)?;
    let pascal = to_pascal_case(name);
    let snake = pascal_to_snake(&pascal);

    if !config.entity.iter().any(|e| e.name == pascal) {
        return Err(format!(
            "{pascal} not found in puerto.toml. Run `puerto generate domain {pascal}` first."
        )
        .into());
    }

    let db = config.project.db;
    let shared_vos = config.value_object.clone();
    let fields = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .map(|e| e.fields.clone())
        .unwrap_or_default();

    write_repository_files(&pascal, &snake, base, db, &fields, &shared_vos)?;
    patch_infra_lib(base, &snake, db)?;

    if db {
        run_migration(
            &format!("create_{snake}_table"),
            base,
            sqlx_bin,
            Some(&create_table_sql(&snake, &fields)),
        )?;
    }

    let repo_label = if db {
        format!("Pg{pascal}Repository")
    } else {
        format!("InMemory{pascal}Repository")
    };
    println!("✓ infrastructure/      — {repo_label}");
    println!();
    println!("  Next: puerto generate presentation {pascal}");
    Ok(())
}
