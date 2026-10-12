use std::path::Path;

use crate::generators::bootstrap::regenerate_bootstrap;
use crate::generators::naming::{entity_ctx, pascal_to_snake, to_pascal_case, write_file};
use crate::generators::render::render;
use crate::patchers::lib_rs::patch_business_lib_use_case;
use crate::patchers::routes_rs::patch_routes_use_cases;

pub fn run_use_case(
    entity: &str,
    action: &str,
    base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let pascal = to_pascal_case(entity);
    let snake = pascal_to_snake(&pascal);
    let uc = action.to_string();
    let uc_pascal = to_pascal_case(&uc);

    crate::validation::validate_use_case_name(&uc)?;

    let trait_path = base.join(format!("business/src/domain/{snake}/use_cases/{uc}.rs"));
    let impl_path = base.join(format!("business/src/application/{snake}/{uc}.rs"));

    // Checked before the first byte is written: re-running this command — or naming one of the
    // scaffolded CRUD use cases — used to replace a working implementation with a `todo!()` stub.
    let config = crate::puerto_toml::read(base)?;
    let declared = config
        .entity
        .iter()
        .find(|e| e.name == pascal)
        .ok_or_else(|| format!("entity '{pascal}' not found in puerto.toml"))?;
    if declared.use_cases.contains(&uc) || trait_path.exists() || impl_path.exists() {
        return Err(format!(
            "use case '{uc}' already exists for {pascal}. Pick another name, or delete its files \
             and its entry in puerto.toml first"
        )
        .into());
    }

    crate::puerto_toml::add_use_case(base, &pascal, &uc)?;

    let uow =
        crate::generators::bootstrap::uses_unit_of_work(declared, config.project.has_events());
    let mut ctx = entity_ctx(&pascal, &snake);
    ctx.insert("uc", &uc);
    ctx.insert("uc_pascal", &uc_pascal);
    ctx.insert("uow", &uow);
    // Protected by default, as the routes: its params carry the caller.
    ctx.insert("actor", &config.project.has_auth());

    write_file(&trait_path, &render("use_case/trait.tera", &ctx)?)?;
    write_file(&impl_path, &render("use_case/impl.tera", &ctx)?)?;

    patch_business_lib_use_case(base, &snake, &uc)?;
    patch_routes_use_cases(base, &pascal, &snake, std::slice::from_ref(&uc))?;
    regenerate_bootstrap(base)?;

    println!("✓ Use case {uc_pascal} added to {pascal} (2 files).");
    println!("  puerto.toml updated + bootstrap.rs regenerated.");
    println!(
        "  {pascal}Api now holds `{uc}` — call it from an endpoint in presentation/src/api/{snake}/routes.rs."
    );

    Ok(())
}
