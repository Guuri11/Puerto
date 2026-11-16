// Phase 11.2 (T-27): `list_*` reads one page, never the whole table.
use super::*;

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn scaffold_crud_producto(name: &str, db: bool) -> PathBuf {
    let dir = temp_dir(name);
    cleanup(&dir);
    setup_puerto_stubs(&dir);
    if db {
        fs::create_dir_all(dir.join("migraciones")).unwrap();
    }
    scaffold::run("Producto", &dir, db, &[], &[]).unwrap();
    dir
}

/// A fresh `puerto nuevo` project (web by default) with a scaffolded `Producto`.
fn web_project_with_producto(name: &str) -> (PathBuf, PathBuf) {
    let dir = temp_dir(name);
    cleanup(&dir);
    fs::create_dir_all(&dir).unwrap();
    let output = generate_project(name, &dir).unwrap();
    scaffold::run_scaffold("Producto", &output, &[], OverwritePolicy::Ask).unwrap();
    (dir, output)
}

#[test]
fn repository_port_reads_a_page() {
    let dir = scaffold_crud_producto("paged_port", false);

    let port = read(&dir, "contextos/producto/dominio/src/repositorio.rs");
    assert!(port.contains(
        "async fn buscar_todos(&self, pagina: PeticionPagina) -> Result<Pagina<Producto>, ErrorRepositorio>;"
    ));
    assert!(port.contains("use puerto_nucleo::{Pagina, PeticionPagina, ErrorRepositorio};"));

    cleanup(&dir);
}

#[test]
fn list_use_case_takes_and_returns_a_page() {
    let dir = scaffold_crud_producto("paged_use_case", false);

    let contract = read(
        &dir,
        "contextos/producto/aplicacion/src/listar_productos.rs",
    );
    assert!(contract.contains("pub pagina: PeticionPagina,"));
    assert!(contract.contains("Result<Pagina<Producto>, ErrorProducto>"));
    let implementation = read(
        &dir,
        "contextos/producto/aplicacion/src/listar_productos.rs",
    );
    assert!(implementation.contains(".buscar_todos(params.pagina)"));
    assert!(implementation.contains("fn deberia_pasar_al_repositorio_la_pagina_pedida()"));

    cleanup(&dir);
}

#[test]
fn postgres_repository_limits_offsets_and_counts() {
    let dir = scaffold_crud_producto("paged_postgres", true);

    let repository = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repository.contains("ORDER BY creado_en DESC, id LIMIT $1 OFFSET $2"));
    assert!(repository.contains("i64::from(pagina.limite())"));
    assert!(repository.contains("i64::from(pagina.desplazamiento())"));
    assert!(
        repository.contains(r#"SELECT COUNT(*) AS "total!" FROM productos WHERE borrado = false"#)
    );
    assert!(repository.contains("Ok(Pagina::new(elementos, total, pagina))"));
    assert!(
        repository.contains(
            "async fn deberia_devolver_la_pagina_pedida_de_una_lista_mayor(pool: PgPool)"
        )
    );

    cleanup(&dir);
}

#[test]
fn inmemory_repository_answers_a_page_of_its_rows() {
    let dir = scaffold_crud_producto("paged_inmemory", false);

    let repository = read(
        &dir,
        "contextos/producto/infraestructura/src/repositorio.rs",
    );
    assert!(repository.contains("Ok(Pagina::new(elementos, total, pagina))"));

    cleanup(&dir);
}

#[test]
fn list_route_reads_limit_and_offset_and_answers_a_page_dto() {
    let dir = scaffold_crud_producto("paged_route", false);

    let routes = read(&dir, "contextos/producto/rest/src/rutas.rs");
    assert!(routes.contains("limite: Query<Option<u32>>, desplazamiento: Query<Option<u32>>"));
    assert!(routes.contains("pagina: PeticionPagina::new(limite.0, desplazamiento.0)"));
    assert!(routes.contains("DtoPaginaProducto::from_dominio(&pagina)"));
    assert!(routes.contains("fn deberia_pasar_limite_y_desplazamiento_al_caso_de_uso()"));
    assert!(routes.contains(r#".get("/productos?limite=5&desplazamiento=10")"#));
    let dto = read(&dir, "contextos/producto/rest/src/dto.rs");
    assert!(dto.contains("pub struct DtoPaginaProducto {"));
    assert!(dto.contains("pub elementos: Vec<DtoProducto>,"));
    assert!(dto.contains("pub total: u64,"));
    let responses = read(&dir, "contextos/producto/rest/src/respuestas.rs");
    assert!(responses.contains("Ok(Json<DtoPaginaProducto>)"));

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
    let (dir, output) = web_project_with_producto("paged_web_slice");

    let port = read(&output, "web/src/domain/producto/repository.ts");
    assert!(port.contains("list(page: PageRequest): Promise<Page<Producto>>;"));
    let adapter = read(
        &output,
        "web/src/infrastructure/producto/producto-repository-http.ts",
    );
    assert!(
        adapter.contains("params: { query: { limite: page.limit, desplazamiento: page.offset } }")
    );
    assert!(adapter.contains("items: data.elementos.map(toProducto)"));
    let fake = read(&output, "web/src/test-support/producto.ts");
    assert!(fake.contains("items: all.slice(offset, offset + limit)"));
    let list = read(
        &output,
        "web/src/presentation/features/producto/containers/producto-list-container.tsx",
    );
    assert!(list.contains("<Pagination"));
    assert!(list.contains("productoRepository.list({ limit: PAGE_SIZE, offset })"));
    let list_test = read(
        &output,
        "web/src/presentation/features/producto/containers/producto-list-container.test.tsx",
    );
    assert!(list_test.contains("should show the next page when Next is pressed"));

    cleanup(&dir);
}
