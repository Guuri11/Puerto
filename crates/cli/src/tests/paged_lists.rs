// Phase 11.2 (T-27): `list_*` reads one page, never the whole table.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn scaffold_crud_product(name: &str, db: bool) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    if db {
        fs::create_dir_all(dir.join("infrastructure/migrations")).unwrap();
    }
    scaffold::run("Product", &dir, db, true, &[], &[]).unwrap();
    dir
}

/// A fresh `puerto new` project (web by default) with a scaffolded `Product`.
fn web_project_with_product(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    scaffold::run_scaffold("Product", &output, &[], OverwritePolicy::Ask).unwrap();
    (dir, output)
}

#[test]
fn new_project_ships_the_page_types_with_a_capped_limit() {
    let dir = temp_dir("paged_template");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("paged-template", &dir).unwrap();

    assert!(read(&output, "business/src/domain/shared.rs").contains("pub mod pagination;"));
    let pagination = read(&output, "business/src/domain/shared/pagination.rs");
    assert!(pagination.contains("pub const DEFAULT_PAGE_LIMIT: u32 = 20;"));
    assert!(pagination.contains("pub const MAX_PAGE_LIMIT: u32 = 100;"));
    assert!(pagination.contains("pub struct PageRequest {\n    limit: u32,\n    offset: u32,\n}"));
    assert!(pagination.contains("pub fn new(limit: Option<u32>, offset: Option<u32>) -> Self"));
    assert!(pagination.contains("pub struct Page<T> {"));
    assert!(pagination.contains("fn should_cap_the_limit_at_the_maximum()"));
    assert!(pagination.contains("fn should_raise_a_zero_limit_to_one()"));
    assert!(pagination.contains("fn should_take_the_defaults_when_nothing_is_asked()"));

    cleanup(&dir);
}

#[test]
fn repository_port_reads_a_page() {
    let dir = scaffold_crud_product("paged_port", false);

    let port = read(&dir, "business/src/domain/product/repository.rs");
    assert!(port.contains(
        "async fn find_all(&self, page: PageRequest) -> Result<Page<Product>, RepositoryError>;"
    ));
    assert!(port.contains("use crate::domain::shared::pagination::{Page, PageRequest};"));

    cleanup(&dir);
}

#[test]
fn list_use_case_takes_and_returns_a_page() {
    let dir = scaffold_crud_product("paged_use_case", false);

    let contract = read(
        &dir,
        "business/src/domain/product/use_cases/list_products.rs",
    );
    assert!(contract.contains("pub page: PageRequest,"));
    assert!(contract.contains("Result<Page<Product>, ProductError>"));
    let implementation = read(&dir, "business/src/application/product/list_products.rs");
    assert!(implementation.contains(".find_all(params.page)"));
    assert!(implementation.contains("fn should_pass_the_requested_page_to_the_repository()"));

    cleanup(&dir);
}

#[test]
fn postgres_repository_limits_offsets_and_counts() {
    let dir = scaffold_crud_product("paged_postgres", true);

    let repository = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repository.contains("ORDER BY created_at DESC, id LIMIT $1 OFFSET $2"));
    assert!(repository.contains("i64::from(page.limit())"));
    assert!(repository.contains("i64::from(page.offset())"));
    assert!(
        repository.contains(r#"SELECT COUNT(*) AS "total!" FROM products WHERE deleted = false"#)
    );
    assert!(repository.contains("Ok(Page::new(items, total, page))"));
    assert!(
        repository
            .contains("async fn should_return_the_requested_page_of_a_larger_list(pool: PgPool)")
    );

    cleanup(&dir);
}

#[test]
fn inmemory_repository_answers_an_empty_page() {
    let dir = scaffold_crud_product("paged_inmemory", false);

    let repository = read(&dir, "infrastructure/src/product/repository.rs");
    assert!(repository.contains("Ok(Page::new(vec![], 0, page))"));

    cleanup(&dir);
}

#[test]
fn list_route_reads_limit_and_offset_and_answers_a_page_dto() {
    let dir = scaffold_crud_product("paged_route", false);

    let routes = read(&dir, "presentation/src/api/product/routes.rs");
    assert!(routes.contains("limit: Query<Option<u32>>, offset: Query<Option<u32>>"));
    assert!(routes.contains("page: PageRequest::new(limit.0, offset.0)"));
    assert!(routes.contains("ProductPageDto::from_domain(&page)"));
    assert!(routes.contains("fn should_pass_limit_and_offset_to_the_use_case()"));
    assert!(routes.contains(r#".get("/products?limit=5&offset=10")"#));
    let dto = read(&dir, "presentation/src/api/product/dto.rs");
    assert!(dto.contains("pub struct ProductPageDto {"));
    assert!(dto.contains("pub items: Vec<ProductDto>,"));
    assert!(dto.contains("pub total: u64,"));
    let responses = read(&dir, "presentation/src/api/product/responses.rs");
    assert!(responses.contains("Ok(Json<ProductPageDto>)"));

    cleanup(&dir);
}

#[test]
fn web_template_ships_the_page_type_and_the_pagination_control() {
    let dir = temp_dir("paged_web_template");
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project("paged-web-template", &dir).unwrap();

    let page = read(&output, "web/src/domain/shared/page.ts");
    assert!(page.contains("export interface Page<T>"));
    assert!(page.contains("export interface PageRequest"));
    assert!(page.contains("export const PAGE_SIZE = 20;"));
    assert!(
        output
            .join("web/src/presentation/shared/ui/pagination.tsx")
            .exists()
    );
    assert!(
        output
            .join("web/src/presentation/shared/ui/pagination.test.tsx")
            .exists()
    );

    cleanup(&dir);
}

#[test]
fn web_slice_lists_a_page_at_a_time() {
    let (dir, output) = web_project_with_product("paged_web_slice");

    let port = read(&output, "web/src/domain/product/repository.ts");
    assert!(port.contains("list(page: PageRequest): Promise<Page<Product>>;"));
    let adapter = read(
        &output,
        "web/src/infrastructure/product/product-repository-http.ts",
    );
    assert!(adapter.contains("params: { query: page }"));
    assert!(adapter.contains("items: data.items.map(toProduct)"));
    let fake = read(&output, "web/src/test-support/product.ts");
    assert!(fake.contains("items: all.slice(offset, offset + limit)"));
    let list = read(
        &output,
        "web/src/presentation/features/product/containers/product-list-container.tsx",
    );
    assert!(list.contains("<Pagination"));
    assert!(list.contains("productRepository.list({ limit: PAGE_SIZE, offset })"));
    let list_test = read(
        &output,
        "web/src/presentation/features/product/containers/product-list-container.test.tsx",
    );
    assert!(list_test.contains("should show the next page when Next is pressed"));

    cleanup(&dir);
}
