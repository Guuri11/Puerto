use std::{fs, path::Path};

// ── Snippet JSON ──────────────────────────────────────────────────────────────
//
// Same content written to both .zed/snippets/rust.json and
// .vscode/puerto.code-snippets — Zed and VS Code share the TextMate format.
//
// Escape notes inside the JSON body strings:
//   \"  → literal double-quote (JSON escape)
//   \\$ → literal dollar sign so snippet engines don't treat SQL $1/$2 as tab stops
//   \t  → tab character (indentation)

pub const SNIPPETS_JSON: &str = r##"{
  "Domain — lib.rs entity block": {
    "prefix": "lib-domain-entity",
    "body": [
      "pub mod ${1:entity} {",
      "\tpub mod errors;",
      "\tpub mod model;",
      "\tpub mod repository;",
      "\tpub mod use_cases {",
      "\t\tpub mod ${2:create};",
      "\t}",
      "}"
    ],
    "description": "Inline domain entity block for business/src/lib.rs."
  },
  "Application — lib.rs entity block": {
    "prefix": "lib-application-entity",
    "body": [
      "pub mod ${1:entity} {",
      "\tpub mod ${2:create};",
      "}"
    ],
    "description": "Inline application entity block for business/src/lib.rs."
  },
  "Domain — model.rs": {
    "prefix": "domain-model",
    "body": [
      "use chrono::NaiveDateTime;",
      "use uuid::Uuid;",
      "",
      "use super::errors::${1:Entity}Error;",
      "",
      "#[derive(Debug, Clone)]",
      "pub struct ${1:Entity}Props {",
      "\tpub name: String,",
      "\t$0",
      "}",
      "",
      "#[derive(Debug, Clone)]",
      "pub struct ${1:Entity} {",
      "\tpub id: Uuid,",
      "\tpub created_at: NaiveDateTime,",
      "\tpub updated_at: NaiveDateTime,",
      "\tpub deleted: bool,",
      "\tpub deleted_at: Option<NaiveDateTime>,",
      "\tpub name: String,",
      "}",
      "",
      "impl ${1:Entity} {",
      "\tpub fn new(props: ${1:Entity}Props) -> Result<Self, ${1:Entity}Error> {",
      "\t\tif props.name.trim().is_empty() {",
      "\t\t\treturn Err(${1:Entity}Error::ValidationError(\"name_empty\".into()));",
      "\t\t}",
      "\t\tlet now = chrono::Utc::now().naive_utc();",
      "\t\tOk(Self {",
      "\t\t\tid: Uuid::new_v4(),",
      "\t\t\tcreated_at: now,",
      "\t\t\tupdated_at: now,",
      "\t\t\tdeleted: false,",
      "\t\t\tdeleted_at: None,",
      "\t\t\tname: props.name,",
      "\t\t})",
      "\t}",
      "",
      "\tpub fn from_repository(data: ${1:Entity}) -> Self {",
      "\t\tdata",
      "\t}",
      "}"
    ],
    "description": "Domain model: Props + Entity struct + new(props) + from_repository()."
  },
  "Domain — errors.rs": {
    "prefix": "domain-errors",
    "body": [
      "use thiserror::Error;",
      "",
      "#[derive(Debug, Error)]",
      "pub enum ${1:Entity}Error {",
      "\t#[error(\"${2:entity}.validation_error.{0}\")]",
      "\tValidationError(String),",
      "\t#[error(\"${2:entity}.not_found\")]",
      "\tNotFound,",
      "\t#[error(\"${2:entity}.repository_error\")]",
      "\tRepositoryError,",
      "}"
    ],
    "description": "Domain errors enum with machine-readable error codes."
  },
  "Domain — repository.rs trait + mock": {
    "prefix": "repository-trait",
    "body": [
      "use async_trait::async_trait;",
      "use uuid::Uuid;",
      "",
      "use super::{errors::${1:Entity}Error, model::${1:Entity}};",
      "",
      "#[async_trait]",
      "pub trait ${1:Entity}RepositoryTrait: Send + Sync {",
      "\tasync fn find_by_id(&self, id: Uuid) -> Result<Option<${1:Entity}>, ${1:Entity}Error>;",
      "\tasync fn save(&self, entity: &${1:Entity}) -> Result<(), ${1:Entity}Error>;",
      "}",
      "",
      "#[cfg(any(test, feature = \"test-utils\"))]",
      "pub mod mocks {",
      "\tuse mockall::mock;",
      "\tuse uuid::Uuid;",
      "\tuse super::*;",
      "",
      "\tmock! {",
      "\t\tpub ${1:Entity}Repository {}",
      "",
      "\t\t#[async_trait]",
      "\t\timpl ${1:Entity}RepositoryTrait for ${1:Entity}Repository {",
      "\t\t\tasync fn find_by_id(&self, id: Uuid) -> Result<Option<${1:Entity}>, ${1:Entity}Error>;",
      "\t\t\tasync fn save(&self, entity: &${1:Entity}) -> Result<(), ${1:Entity}Error>;",
      "\t\t}",
      "\t}",
      "}"
    ],
    "description": "Repository port trait + mockall mock in pub mod mocks."
  },
  "Domain — use_cases/<action>.rs": {
    "prefix": "domain-use-case",
    "body": [
      "use async_trait::async_trait;",
      "",
      "use crate::domain::${1:entity}::{errors::${2:Entity}Error, model::${2:Entity}};",
      "",
      "#[derive(Debug, Clone)]",
      "pub struct ${3:Action}${2:Entity}Params {",
      "\tpub name: String,",
      "\t$0",
      "}",
      "",
      "#[async_trait]",
      "pub trait ${3:Action}${2:Entity}UseCaseTrait: Send + Sync {",
      "\tasync fn execute(&self, params: ${3:Action}${2:Entity}Params) -> Result<${2:Entity}, ${2:Entity}Error>;",
      "}"
    ],
    "description": "Use case Params struct + UseCaseTrait."
  },
  "Application — use case impl + tests": {
    "prefix": "app-use-case",
    "body": [
      "use std::sync::Arc;",
      "",
      "use async_trait::async_trait;",
      "",
      "use crate::domain::${1:entity}::{",
      "\terrors::${2:Entity}Error,",
      "\tmodel::{${2:Entity}, ${2:Entity}Props},",
      "\trepository::${2:Entity}RepositoryTrait,",
      "\tuse_cases::${3:action}::{${4:Action}${2:Entity}Params, ${4:Action}${2:Entity}UseCaseTrait},",
      "};",
      "use crate::domain::logger::LoggerTrait;",
      "",
      "pub struct ${4:Action}${2:Entity}UseCaseImpl {",
      "\tpub repository: Arc<dyn ${2:Entity}RepositoryTrait>,",
      "\tpub logger: Arc<dyn LoggerTrait>,",
      "}",
      "",
      "#[async_trait]",
      "impl ${4:Action}${2:Entity}UseCaseTrait for ${4:Action}${2:Entity}UseCaseImpl {",
      "\tasync fn execute(&self, params: ${4:Action}${2:Entity}Params) -> Result<${2:Entity}, ${2:Entity}Error> {",
      "\t\tself.logger.info(&format!(\"${3:action}: {}\", params.name));",
      "\t\tlet entity = ${2:Entity}::new(${2:Entity}Props { name: params.name })?;",
      "\t\tself.repository.save(&entity).await?;",
      "\t\tself.logger.info(&format!(\"${2:Entity} created: {}\", entity.id));",
      "\t\tOk(entity)",
      "\t}",
      "}",
      "",
      "#[cfg(test)]",
      "mod tests {",
      "\tuse super::*;",
      "\tuse crate::domain::${1:entity}::repository::mocks::Mock${2:Entity}Repository;",
      "\tuse crate::domain::logger::mocks::MockLogger;",
      "",
      "\tfn silent_logger() -> MockLogger {",
      "\t\tlet mut mock = MockLogger::new();",
      "\t\tmock.expect_info().returning(|_| ());",
      "\t\tmock",
      "\t}",
      "",
      "\t#[tokio::test]",
      "\tasync fn should_create_${1:entity}_when_name_is_valid() {",
      "\t\t// Arrange",
      "\t\tlet mut mock_repo = Mock${2:Entity}Repository::new();",
      "\t\tmock_repo.expect_save().returning(|_| Ok(()));",
      "\t\tlet use_case = ${4:Action}${2:Entity}UseCaseImpl {",
      "\t\t\trepository: Arc::new(mock_repo),",
      "\t\t\tlogger: Arc::new(silent_logger()),",
      "\t\t};",
      "",
      "\t\t// Act",
      "\t\tlet result = use_case",
      "\t\t\t.execute(${4:Action}${2:Entity}Params { name: \"example\".into() })",
      "\t\t\t.await;",
      "",
      "\t\t// Assert",
      "\t\tassert!(result.is_ok());",
      "\t\tassert_eq!(result.unwrap().name, \"example\");",
      "\t}",
      "}"
    ],
    "description": "Use case impl with LoggerTrait + repository injection + unit tests."
  },
  "Infrastructure — lib.rs entity block (InMemory)": {
    "prefix": "lib-infra-entity",
    "body": [
      "pub mod ${1:entity} {",
      "\tpub mod repository;",
      "}"
    ],
    "description": "Infrastructure entity block for infrastructure/src/lib.rs (InMemory)."
  },
  "Infrastructure — lib.rs entity block (SQLx)": {
    "prefix": "lib-infra-entity-db",
    "body": [
      "pub mod ${1:entity} {",
      "\tpub mod entity;",
      "\tpub mod repository;",
      "}"
    ],
    "description": "Infrastructure entity block for infrastructure/src/lib.rs (SQLx/Postgres)."
  },
  "Infrastructure — entity.rs (DB row struct)": {
    "prefix": "persistence-entity",
    "body": [
      "use chrono::NaiveDateTime;",
      "use serde::{Deserialize, Serialize};",
      "use sqlx::FromRow;",
      "use uuid::Uuid;",
      "",
      "use business::domain::${1:entity}::{errors::${2:Entity}Error, model::${2:Entity}};",
      "",
      "#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]",
      "pub struct ${2:Entity}Db {",
      "\tpub id: Uuid,",
      "\tpub created_at: NaiveDateTime,",
      "\tpub updated_at: NaiveDateTime,",
      "\tpub deleted: bool,",
      "\tpub deleted_at: Option<NaiveDateTime>,",
      "\tpub name: String,",
      "\t$0",
      "}",
      "",
      "impl TryFrom<${2:Entity}Db> for ${2:Entity} {",
      "\ttype Error = ${2:Entity}Error;",
      "",
      "\tfn try_from(row: ${2:Entity}Db) -> Result<Self, Self::Error> {",
      "\t\tOk(Self::from_repository(${2:Entity} {",
      "\t\t\tid: row.id,",
      "\t\t\tcreated_at: row.created_at,",
      "\t\t\tupdated_at: row.updated_at,",
      "\t\t\tdeleted: row.deleted,",
      "\t\t\tdeleted_at: row.deleted_at,",
      "\t\t\tname: row.name,",
      "\t\t}))",
      "\t}",
      "}",
      "",
      "impl From<&${2:Entity}> for ${2:Entity}Db {",
      "\tfn from(entity: &${2:Entity}) -> Self {",
      "\t\tSelf {",
      "\t\t\tid: entity.id,",
      "\t\t\tcreated_at: entity.created_at,",
      "\t\t\tupdated_at: entity.updated_at,",
      "\t\t\tdeleted: entity.deleted,",
      "\t\t\tdeleted_at: entity.deleted_at,",
      "\t\t\tname: entity.name.clone(),",
      "\t\t}",
      "\t}",
      "}"
    ],
    "description": "DB row struct (FromRow) + TryFrom<EntityDb>/From<&Entity> conversions."
  },
  "Infrastructure — repository.rs (SQLx/Postgres)": {
    "prefix": "persistence-repo",
    "body": [
      "use async_trait::async_trait;",
      "use sqlx::PgPool;",
      "use uuid::Uuid;",
      "",
      "use business::domain::${1:entity}::{",
      "\terrors::${2:Entity}Error,",
      "\tmodel::${2:Entity},",
      "\trepository::${2:Entity}RepositoryTrait,",
      "};",
      "",
      "use super::entity::${2:Entity}Db;",
      "",
      "pub struct Pg${2:Entity}Repository {",
      "\tpub pool: PgPool,",
      "}",
      "",
      "impl Pg${2:Entity}Repository {",
      "\tpub fn new(pool: PgPool) -> Self {",
      "\t\tSelf { pool }",
      "\t}",
      "}",
      "",
      "#[async_trait]",
      "impl ${2:Entity}RepositoryTrait for Pg${2:Entity}Repository {",
      "\tasync fn find_by_id(&self, id: Uuid) -> Result<Option<${2:Entity}>, ${2:Entity}Error> {",
      "\t\tlet row = sqlx::query_as!(${2:Entity}Db,",
      "\t\t\t\"SELECT id, created_at, updated_at, deleted, deleted_at, name FROM ${1:entity}s WHERE id = \\$1 AND deleted = false\",",
      "\t\t\tid",
      "\t\t)",
      "\t\t.fetch_optional(&self.pool)",
      "\t\t.await",
      "\t\t.map_err(|_| ${2:Entity}Error::RepositoryError)?;",
      "",
      "\t\trow.map(|r| r.try_into()).transpose()",
      "\t}",
      "",
      "\tasync fn save(&self, entity: &${2:Entity}) -> Result<(), ${2:Entity}Error> {",
      "\t\tlet db = ${2:Entity}Db::from(entity);",
      "\t\tsqlx::query!(",
      "\t\t\t\"INSERT INTO ${1:entity}s (id, created_at, updated_at, deleted, deleted_at, name) VALUES (\\$1, \\$2, \\$3, \\$4, \\$5, \\$6) ON CONFLICT (id) DO UPDATE SET updated_at = \\$3, deleted = \\$4, deleted_at = \\$5, name = \\$6\",",
      "\t\t\tdb.id, db.created_at, db.updated_at, db.deleted, db.deleted_at, db.name",
      "\t\t)",
      "\t\t.execute(&self.pool)",
      "\t\t.await",
      "\t\t.map_err(|_| ${2:Entity}Error::RepositoryError)?;",
      "\t\tOk(())",
      "\t}",
      "}",
      "$0"
    ],
    "description": "PgEntityRepository: find_by_id + save with SQLx query macros."
  },
  "Presentation — api/<entity>.rs module decls": {
    "prefix": "lib-presentation-entity",
    "body": [
      "pub mod dto;",
      "pub mod error_mapper;",
      "pub mod responses;",
      "pub mod routes;"
    ],
    "description": "Module declarations for presentation/src/api/<entity>.rs."
  },
  "Presentation — dto.rs response DTO": {
    "prefix": "poem-dto",
    "body": [
      "use business::domain::${1:entity}::model::${2:Entity};",
      "use poem_openapi::Object;",
      "use uuid::Uuid;",
      "",
      "#[derive(Debug, Object)]",
      "pub struct ${2:Entity}Dto {",
      "\tpub id: Uuid,",
      "\tpub name: String,",
      "\t$0",
      "}",
      "",
      "impl ${2:Entity}Dto {",
      "\tpub fn from_domain(entity: &${2:Entity}) -> Self {",
      "\t\tSelf {",
      "\t\t\tid: entity.id,",
      "\t\t\tname: entity.name.clone(),",
      "\t\t}",
      "\t}",
      "}"
    ],
    "description": "OpenAPI response DTO with from_domain() mapping."
  },
  "Presentation — dto.rs request DTO": {
    "prefix": "poem-request-dto",
    "body": [
      "#[derive(Debug, Object)]",
      "pub struct ${1:Action}${2:Entity}Request {",
      "\tpub name: String,",
      "\t$0",
      "}"
    ],
    "description": "OpenAPI request DTO struct."
  },
  "Presentation — responses.rs ApiResponse enum": {
    "prefix": "poem-response-enum",
    "body": [
      "use crate::api::{error::ErrorResponse, ${1:entity}::dto::${2:Entity}Dto};",
      "use poem::http::StatusCode;",
      "use poem_openapi::{ApiResponse, payload::Json};",
      "",
      "#[derive(ApiResponse)]",
      "pub enum ${3:Action}${2:Entity}Response {",
      "\t#[oai(status = 201)]",
      "\tCreated(Json<${2:Entity}Dto>),",
      "\t#[oai(status = 400)]",
      "\tBadRequest(Json<ErrorResponse>),",
      "\t#[oai(status = 404)]",
      "\tNotFound(Json<ErrorResponse>),",
      "\t#[oai(status = 500)]",
      "\tInternalError(Json<ErrorResponse>),",
      "}",
      "",
      "impl ${3:Action}${2:Entity}Response {",
      "\tpub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {",
      "\t\tmatch status {",
      "\t\t\tStatusCode::BAD_REQUEST => Self::BadRequest(error),",
      "\t\t\tStatusCode::NOT_FOUND => Self::NotFound(error),",
      "\t\t\t_ => Self::InternalError(error),",
      "\t\t}",
      "\t}",
      "}"
    ],
    "description": "ApiResponse enum with from_status() helper."
  },
  "Presentation — error_mapper.rs": {
    "prefix": "poem-error-mapper",
    "body": [
      "use business::domain::${1:entity}::errors::${2:Entity}Error;",
      "use poem::http::StatusCode;",
      "use poem_openapi::payload::Json;",
      "",
      "use crate::api::error::{ErrorResponse, IntoErrorResponse};",
      "",
      "impl IntoErrorResponse for ${2:Entity}Error {",
      "\tfn into_error_response(self) -> (StatusCode, Json<ErrorResponse>) {",
      "\t\tlet (status, message) = match &self {",
      "\t\t\t${2:Entity}Error::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),",
      "\t\t\t${2:Entity}Error::NotFound => (StatusCode::NOT_FOUND, self.to_string()),",
      "\t\t\t${2:Entity}Error::RepositoryError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),",
      "\t\t};",
      "\t\t(status, Json(ErrorResponse { name: \"${1:entity}_error\".into(), message }))",
      "\t}",
      "}"
    ],
    "description": "IntoErrorResponse impl mapping EntityError to (StatusCode, Json<ErrorResponse>)."
  },
  "Presentation — routes.rs Api struct + endpoint": {
    "prefix": "poem-api-struct",
    "body": [
      "use std::sync::Arc;",
      "",
      "use business::{",
      "\tapplication::${1:entity}::${3:action}::${4:Action}${2:Entity}UseCaseImpl,",
      "\tdomain::${1:entity}::use_cases::${3:action}::{${4:Action}${2:Entity}Params, ${4:Action}${2:Entity}UseCaseTrait},",
      "};",
      "use poem_openapi::{OpenApi, payload::Json};",
      "",
      "use crate::api::error::IntoErrorResponse;",
      "use crate::api::${1:entity}::dto::{${4:Action}${2:Entity}Request, ${2:Entity}Dto};",
      "use crate::api::${1:entity}::responses::${4:Action}${2:Entity}Response;",
      "",
      "pub struct ${2:Entity}Api {",
      "\tpub ${3:action}: Arc<${4:Action}${2:Entity}UseCaseImpl>,",
      "}",
      "",
      "#[OpenApi]",
      "impl ${2:Entity}Api {",
      "\t/// ${4:Action} a ${2:Entity}",
      "\t#[oai(path = \"/${1:entity}s\", method = \"post\")]",
      "\tasync fn ${3:action}(&self, body: Json<${4:Action}${2:Entity}Request>) -> ${4:Action}${2:Entity}Response {",
      "\t\tmatch self.${3:action}.execute(${4:Action}${2:Entity}Params { name: body.name.clone() }).await {",
      "\t\t\tOk(entity) => ${4:Action}${2:Entity}Response::Created(Json(${2:Entity}Dto::from_domain(&entity))),",
      "\t\t\tErr(err) => {",
      "\t\t\t\tlet (status, error) = err.into_error_response();",
      "\t\t\t\t${4:Action}${2:Entity}Response::from_status(status, error)",
      "\t\t\t}",
      "\t\t}",
      "\t}",
      "}"
    ],
    "description": "Poem OpenAPI Api struct with use case field and POST endpoint."
  },
  "Presentation — single endpoint handler": {
    "prefix": "poem-endpoint",
    "body": [
      "/// ${1:description}",
      "#[oai(path = \"/${2:entity}s\", method = \"${3:post}\")]",
      "async fn ${4:action}(&self, body: Json<${5:Action}${6:Entity}Request>) -> ${5:Action}${6:Entity}Response {",
      "\tmatch self.${4:action}.execute(${5:Action}${6:Entity}Params { name: body.name.clone() }).await {",
      "\t\tOk(entity) => ${5:Action}${6:Entity}Response::Created(Json(${6:Entity}Dto::from_domain(&entity))),",
      "\t\tErr(err) => {",
      "\t\t\tlet (status, error) = err.into_error_response();",
      "\t\t\t${5:Action}${6:Entity}Response::from_status(status, error)",
      "\t\t}",
      "\t}",
      "}"
    ],
    "description": "#[oai] endpoint handler: execute → map response."
  },
  "Test — cfg(test) block": {
    "prefix": "cfg-test",
    "body": [
      "#[cfg(test)]",
      "mod tests {",
      "\tuse super::*;",
      "\tuse std::sync::Arc;",
      "",
      "\t$0",
      "}"
    ],
    "description": "Test module with standard imports."
  },
  "Test — tokio async test (AAA)": {
    "prefix": "should-do-test",
    "body": [
      "#[tokio::test]",
      "async fn should_${1:expected}_when_${2:condition}() {",
      "\t// Arrange",
      "\t$0",
      "",
      "\t// Act",
      "",
      "\t// Assert",
      "}"
    ],
    "description": "Async tokio test with AAA pattern and business-focused naming."
  },
  "Test — Object Mother": {
    "prefix": "object-mother",
    "body": [
      "pub struct ${1:Entity}Mother;",
      "",
      "impl ${1:Entity}Mother {",
      "\tpub fn random() -> ${1:Entity} {",
      "\t\tSelf::builder().build()",
      "\t}",
      "",
      "\tpub fn builder() -> ${1:Entity}Builder {",
      "\t\t${1:Entity}Builder::new()",
      "\t}",
      "}",
      "",
      "pub struct ${1:Entity}Builder {",
      "\tname: Option<String>,",
      "\t$0",
      "}",
      "",
      "impl ${1:Entity}Builder {",
      "\tpub fn new() -> Self {",
      "\t\tSelf { name: None }",
      "\t}",
      "",
      "\tpub fn with_name(mut self, name: &str) -> Self {",
      "\t\tself.name = Some(name.to_string());",
      "\t\tself",
      "\t}",
      "",
      "\tpub fn build(self) -> ${1:Entity} {",
      "\t\t${1:Entity}::new(${1:Entity}Props {",
      "\t\t\tname: self.name.unwrap_or_else(|| \"example\".to_string()),",
      "\t\t})",
      "\t\t.expect(\"${1:Entity}Mother: failed to build valid ${1:Entity}\")",
      "\t}",
      "}",
      "",
      "impl Default for ${1:Entity}Builder {",
      "\tfn default() -> Self {",
      "\t\tSelf::new()",
      "\t}",
      "}"
    ],
    "description": "Object Mother with builder pattern: random(), with_name(), build()."
  },
  "Test — SQLx integration test": {
    "prefix": "sqlx-test",
    "body": [
      "#[sqlx::test(migrations = \"migrations\")]",
      "async fn should_${1:expected}_when_${2:condition}(pool: PgPool) {",
      "\t// Arrange",
      "\tlet repo = Pg${3:Entity}Repository::new(pool.clone());",
      "\t$0",
      "",
      "\t// Act",
      "",
      "\t// Assert",
      "}"
    ],
    "description": "#[sqlx::test] integration test with real Postgres pool and migrations."
  },
  "Test — SQLx repository test module": {
    "prefix": "sqlx-repo-test-module",
    "body": [
      "#[cfg(test)]",
      "mod integration_tests {",
      "\tuse super::*;",
      "\tuse business::domain::${1:entity}::model::${2:Entity}Props;",
      "",
      "\tasync fn seed(pool: &PgPool, name: &str) -> ${2:Entity} {",
      "\t\tlet entity = ${2:Entity}::new(${2:Entity}Props { name: name.to_string() }).unwrap();",
      "\t\tPg${2:Entity}Repository::new(pool.clone()).save(&entity).await.unwrap();",
      "\t\tentity",
      "\t}",
      "",
      "\t#[sqlx::test(migrations = \"migrations\")]",
      "\tasync fn should_persist_and_retrieve_by_id(pool: PgPool) {",
      "\t\t// Arrange",
      "\t\tlet entity = seed(&pool, \"example\").await;",
      "",
      "\t\t// Act",
      "\t\tlet found = Pg${2:Entity}Repository::new(pool)",
      "\t\t\t.find_by_id(entity.id)",
      "\t\t\t.await",
      "\t\t\t.unwrap()",
      "\t\t\t.unwrap();",
      "",
      "\t\t// Assert",
      "\t\tassert_eq!(found.id, entity.id);",
      "\t\tassert_eq!(found.name, entity.name);",
      "\t}",
      "",
      "\t#[sqlx::test(migrations = \"migrations\")]",
      "\tasync fn should_return_none_for_nonexistent_id(pool: PgPool) {",
      "\t\t// Act",
      "\t\tlet result = Pg${2:Entity}Repository::new(pool)",
      "\t\t\t.find_by_id(Uuid::new_v4())",
      "\t\t\t.await",
      "\t\t\t.unwrap();",
      "",
      "\t\t// Assert",
      "\t\tassert!(result.is_none());",
      "\t}",
      "}"
    ],
    "description": "Full #[cfg(test)] integration test module for PgEntityRepository with seed helper."
  }
}
"##;

pub const SQL_SNIPPETS_JSON: &str = r##"{
  "Puerto — Create table": {
    "prefix": "migration-create-table",
    "scope": "sql",
    "body": [
      "CREATE TABLE ${1:entities} (",
      "    id UUID PRIMARY KEY,",
      "    name TEXT NOT NULL,",
      "    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),",
      "    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),",
      "    deleted BOOLEAN NOT NULL DEFAULT FALSE,",
      "    deleted_at TIMESTAMPTZ",
      ");",
      "$0"
    ],
    "description": "CREATE TABLE with Puerto standard columns (id, name, timestamps, soft-delete)."
  },
  "Puerto — Add column": {
    "prefix": "migration-add-column",
    "scope": "sql",
    "body": [
      "ALTER TABLE ${1:entities} ADD COLUMN ${2:column_name} ${3:TEXT} NOT NULL${4: DEFAULT ''};",
      "$0"
    ],
    "description": "ALTER TABLE ADD COLUMN."
  },
  "Puerto — Insert": {
    "prefix": "sql-insert",
    "scope": "sql",
    "body": [
      "INSERT INTO ${1:entities} (id, name, created_at, updated_at, deleted, deleted_at)",
      "VALUES (\\$1, \\$2, \\$3, \\$4, \\$5, \\$6)",
      "ON CONFLICT (id) DO UPDATE",
      "    SET name = \\$2, updated_at = \\$4;",
      "$0"
    ],
    "description": "INSERT with Puerto standard columns + upsert."
  },
  "Puerto — Update": {
    "prefix": "sql-update",
    "scope": "sql",
    "body": [
      "UPDATE ${1:entities}",
      "SET ${2:name} = \\$2, updated_at = NOW()",
      "WHERE id = \\$1 AND deleted = false;",
      "$0"
    ],
    "description": "UPDATE with Puerto soft-delete guard."
  },
  "Puerto — Soft delete": {
    "prefix": "sql-soft-delete",
    "scope": "sql",
    "body": [
      "UPDATE ${1:entities}",
      "SET deleted = true, deleted_at = NOW(), updated_at = NOW()",
      "WHERE id = \\$1 AND deleted = false;",
      "$0"
    ],
    "description": "Soft delete by id — sets deleted = true and deleted_at."
  }
}
"##;

// ── Writers ───────────────────────────────────────────────────────────────────

fn write_file(path: &Path, content: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

/// Write snippet files for the given IDE(s) into `base`.
/// `ide = None` → writes both Zed and VS Code files.
pub fn apply(base: &Path, ide: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let write_zed = ide.is_none_or(|i| i == "zed");
    let write_vscode = ide.is_none_or(|i| i == "vscode");

    if write_zed {
        write_file(&base.join(".zed/snippets/rust.json"), SNIPPETS_JSON)?;
        write_file(&base.join(".zed/snippets/sql.json"), SQL_SNIPPETS_JSON)?;
        println!(
            "✓ .zed/snippets/rust.json + sql.json  (Zed — loaded automatically from project root)"
        );
    }
    if write_vscode {
        write_file(&base.join(".vscode/puerto.code-snippets"), SNIPPETS_JSON)?;
        write_file(
            &base.join(".vscode/puerto.sql.code-snippets"),
            SQL_SNIPPETS_JSON,
        )?;
        println!(
            "✓ .vscode/puerto.code-snippets + puerto.sql.code-snippets  (VS Code — loaded automatically)"
        );
        println!(
            "  nvim+LuaSnip: require(\"luasnip.loaders.from_vscode\").lazy_load({{ paths = {{ \"./.vscode\" }} }})"
        );
    }

    Ok(())
}

/// `puerto generate snippets [--ide <ide>]`
pub fn run(base: &Path, ide: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(i) = ide {
        if !matches!(i, "zed" | "vscode") {
            return Err(format!("unknown IDE '{i}' — supported values: zed, vscode").into());
        }
    }
    apply(base, ide)
}
