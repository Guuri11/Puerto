use std::path::Path;

use crate::generators::bootstrap::regenerate_bootstrap;
use crate::generators::naming::{apply, pascal_to_snake, to_pascal_case, write_file};
use crate::patchers::api_rs::patch_api_rs;

pub(crate) const DTO: &str = r#"use business::domain::{snake}::model::{Pascal};
use poem_openapi::Object;
use uuid::Uuid;

#[derive(Debug, Object)]
pub struct {Pascal}Dto {
    pub id: Uuid,
    pub name: String,
}

impl {Pascal}Dto {
    pub fn from_domain(entity: &{Pascal}) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
        }
    }
}

#[derive(Debug, Object)]
pub struct Create{Pascal}Request {
    pub name: String,
}
"#;

const CRUD_DTO: &str = r#"use business::domain::{snake}::model::{Pascal};
use poem_openapi::Object;
use uuid::Uuid;

#[derive(Debug, Object)]
pub struct {Pascal}Dto {
    pub id: Uuid,
    pub name: String,
}

impl {Pascal}Dto {
    pub fn from_domain(entity: &{Pascal}) -> Self {
        Self {
            id: entity.id,
            name: entity.name.clone(),
        }
    }
}

#[derive(Debug, Object)]
pub struct Create{Pascal}Request {
    pub name: String,
}

#[derive(Debug, Object)]
pub struct Update{Pascal}Request {
    pub name: String,
}
"#;

pub(crate) const RESPONSES: &str = r#"use crate::api::{error::ErrorResponse, {snake}::dto::{Pascal}Dto};
use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};

#[derive(ApiResponse)]
pub enum Create{Pascal}Response {
    #[oai(status = 201)]
    Created(Json<{Pascal}Dto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 409)]
    Conflict(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Create{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            StatusCode::CONFLICT => Self::Conflict(error),
            _ => Self::InternalError(error),
        }
    }
}
"#;

const CRUD_RESPONSES: &str = r#"use crate::api::{error::ErrorResponse, {snake}::dto::{Pascal}Dto};
use poem::http::StatusCode;
use poem_openapi::{ApiResponse, payload::Json};

#[derive(ApiResponse)]
pub enum Create{Pascal}Response {
    #[oai(status = 201)]
    Created(Json<{Pascal}Dto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 409)]
    Conflict(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Create{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            StatusCode::CONFLICT => Self::Conflict(error),
            _ => Self::InternalError(error),
        }
    }
}

#[derive(ApiResponse)]
pub enum Get{Pascal}Response {
    #[oai(status = 200)]
    Ok(Json<{Pascal}Dto>),
    #[oai(status = 404)]
    NotFound(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Get{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::NOT_FOUND => Self::NotFound(error),
            _ => Self::InternalError(error),
        }
    }
}

#[derive(ApiResponse)]
pub enum List{Pascal}Response {
    #[oai(status = 200)]
    Ok(Json<Vec<{Pascal}Dto>>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl List{Pascal}Response {
    pub fn from_status(_status: StatusCode, error: Json<ErrorResponse>) -> Self {
        Self::InternalError(error)
    }
}

#[derive(ApiResponse)]
pub enum Update{Pascal}Response {
    #[oai(status = 200)]
    Ok(Json<{Pascal}Dto>),
    #[oai(status = 400)]
    BadRequest(Json<ErrorResponse>),
    #[oai(status = 404)]
    NotFound(Json<ErrorResponse>),
    #[oai(status = 409)]
    Conflict(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Update{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest(error),
            StatusCode::NOT_FOUND => Self::NotFound(error),
            StatusCode::CONFLICT => Self::Conflict(error),
            _ => Self::InternalError(error),
        }
    }
}

#[derive(ApiResponse)]
pub enum Delete{Pascal}Response {
    #[oai(status = 204)]
    NoContent,
    #[oai(status = 404)]
    NotFound(Json<ErrorResponse>),
    #[oai(status = 500)]
    InternalError(Json<ErrorResponse>),
}

impl Delete{Pascal}Response {
    pub fn from_status(status: StatusCode, error: Json<ErrorResponse>) -> Self {
        match status {
            StatusCode::NOT_FOUND => Self::NotFound(error),
            _ => Self::InternalError(error),
        }
    }
}
"#;

pub(crate) const ERROR_MAPPER: &str = r#"use business::domain::{snake}::errors::{Pascal}Error;
use poem::http::StatusCode;
use poem_openapi::payload::Json;

use crate::api::error::{ErrorResponse, IntoErrorResponse};

impl IntoErrorResponse for {Pascal}Error {
    fn into_error_response(self) -> (StatusCode, Json<ErrorResponse>) {
        let (status, message) = match &self {
            {Pascal}Error::ValidationError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            {Pascal}Error::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            {Pascal}Error::Duplicate => (StatusCode::CONFLICT, self.to_string()),
            {Pascal}Error::RepositoryError => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            {Pascal}Error::Unknown => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (
            status,
            Json(ErrorResponse {
                name: "{snake}_error".into(),
                message,
            }),
        )
    }
}
"#;

pub(crate) const ROUTES: &str = r#"use std::sync::Arc;

use business::{
    application::{snake}::create_{snake}::Create{Pascal}UseCaseImpl,
    domain::{
        {snake}::use_cases::create_{snake}::{Create{Pascal}Params, Create{Pascal}UseCaseTrait},
        logger::LoggerTrait,
    },
};
use poem_openapi::{OpenApi, payload::Json};

use crate::api::error::IntoErrorResponse;
use crate::api::{snake}::dto::{Create{Pascal}Request, {Pascal}Dto};
use crate::api::{snake}::responses::Create{Pascal}Response;

pub struct {Pascal}Api {
    pub create_{snake}: Arc<Create{Pascal}UseCaseImpl>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[OpenApi]
impl {Pascal}Api {
    /// Create a new {Pascal}
    #[oai(path = "/{snake}s", method = "post")]
    async fn create(&self, body: Json<Create{Pascal}Request>) -> Create{Pascal}Response {
        match self
            .create_{snake}
            .execute(Create{Pascal}Params { name: body.name.clone() })
            .await
        {
            Ok(entity) => Create{Pascal}Response::Created(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("create_{snake} error: {}", error.0.message));
                Create{Pascal}Response::from_status(status, error)
            }
        }
    }
}
"#;

const CRUD_ROUTES: &str = r#"use std::sync::Arc;

use business::{
    application::{snake}::{
        create_{snake}::Create{Pascal}UseCaseImpl,
        delete_{snake}::Delete{Pascal}UseCaseImpl,
        get_{snake}::Get{Pascal}UseCaseImpl,
        list_{snake}::List{Pascal}UseCaseImpl,
        update_{snake}::Update{Pascal}UseCaseImpl,
    },
    domain::{
        {snake}::use_cases::{
            create_{snake}::{Create{Pascal}Params, Create{Pascal}UseCaseTrait},
            delete_{snake}::{Delete{Pascal}Params, Delete{Pascal}UseCaseTrait},
            get_{snake}::{Get{Pascal}Params, Get{Pascal}UseCaseTrait},
            list_{snake}::{List{Pascal}Params, List{Pascal}UseCaseTrait},
            update_{snake}::{Update{Pascal}Params, Update{Pascal}UseCaseTrait},
        },
        logger::LoggerTrait,
    },
};
use poem_openapi::{OpenApi, param::Path, payload::Json};
use uuid::Uuid;

use crate::api::error::IntoErrorResponse;
use crate::api::{snake}::dto::{Create{Pascal}Request, Update{Pascal}Request, {Pascal}Dto};
use crate::api::{snake}::responses::{
    Create{Pascal}Response, Delete{Pascal}Response, Get{Pascal}Response, List{Pascal}Response,
    Update{Pascal}Response,
};

pub struct {Pascal}Api {
    pub create_{snake}: Arc<Create{Pascal}UseCaseImpl>,
    pub get_{snake}: Arc<Get{Pascal}UseCaseImpl>,
    pub list_{snake}: Arc<List{Pascal}UseCaseImpl>,
    pub update_{snake}: Arc<Update{Pascal}UseCaseImpl>,
    pub delete_{snake}: Arc<Delete{Pascal}UseCaseImpl>,
    pub logger: Arc<dyn LoggerTrait>,
}

#[OpenApi]
impl {Pascal}Api {
    /// Create a new {Pascal}
    #[oai(path = "/{snake}s", method = "post")]
    async fn create(&self, body: Json<Create{Pascal}Request>) -> Create{Pascal}Response {
        match self
            .create_{snake}
            .execute(Create{Pascal}Params { name: body.name.clone() })
            .await
        {
            Ok(entity) => Create{Pascal}Response::Created(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("create_{snake} error: {}", error.0.message));
                Create{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// Get a {Pascal} by ID
    #[oai(path = "/{snake}s/:id", method = "get")]
    async fn get_by_id(&self, id: Path<Uuid>) -> Get{Pascal}Response {
        match self
            .get_{snake}
            .execute(Get{Pascal}Params { id: id.0 })
            .await
        {
            Ok(entity) => Get{Pascal}Response::Ok(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("get_{snake} error: {}", error.0.message));
                Get{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// List all {Pascal}s
    #[oai(path = "/{snake}s", method = "get")]
    async fn list(&self) -> List{Pascal}Response {
        match self.list_{snake}.execute(List{Pascal}Params).await {
            Ok(entities) => {
                List{Pascal}Response::Ok(Json(entities.iter().map({Pascal}Dto::from_domain).collect()))
            }
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.error(&format!("list_{snake} error: {}", error.0.message));
                List{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// Update a {Pascal}
    #[oai(path = "/{snake}s/:id", method = "put")]
    async fn update(&self, id: Path<Uuid>, body: Json<Update{Pascal}Request>) -> Update{Pascal}Response {
        match self
            .update_{snake}
            .execute(Update{Pascal}Params {
                id: id.0,
                name: body.name.clone(),
            })
            .await
        {
            Ok(entity) => Update{Pascal}Response::Ok(Json({Pascal}Dto::from_domain(&entity))),
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("update_{snake} error: {}", error.0.message));
                Update{Pascal}Response::from_status(status, error)
            }
        }
    }

    /// Delete a {Pascal}
    #[oai(path = "/{snake}s/:id", method = "delete")]
    async fn delete(&self, id: Path<Uuid>) -> Delete{Pascal}Response {
        match self
            .delete_{snake}
            .execute(Delete{Pascal}Params { id: id.0 })
            .await
        {
            Ok(()) => Delete{Pascal}Response::NoContent,
            Err(err) => {
                let (status, error) = err.into_error_response();
                self.logger.warn(&format!("delete_{snake} error: {}", error.0.message));
                Delete{Pascal}Response::from_status(status, error)
            }
        }
    }
}
"#;

pub fn write_presentation_files(
    pascal: &str,
    snake: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    write_file(
        &base.join(format!("presentation/src/api/{snake}.rs")),
        "pub mod dto;\npub mod error_mapper;\npub mod responses;\npub mod routes;\n",
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/dto.rs")),
        &apply(CRUD_DTO, pascal, snake),
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/responses.rs")),
        &apply(CRUD_RESPONSES, pascal, snake),
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/error_mapper.rs")),
        &apply(ERROR_MAPPER, pascal, snake),
    )?;
    write_file(
        &base.join(format!("presentation/src/api/{snake}/routes.rs")),
        &apply(CRUD_ROUTES, pascal, snake),
    )?;
    Ok(())
}

pub fn run_generate_presentation(
    name: &str,
    base: &Path,
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

    write_presentation_files(&pascal, &snake, base)?;
    patch_api_rs(base, &snake)?;
    regenerate_bootstrap(base)?;

    println!("✓ presentation/        — routes, dto, responses, error_mapper");
    println!("✓ bootstrap.rs         — regenerated");
    println!();
    println!("  All layers wired. Run `make run` to start.");
    Ok(())
}

