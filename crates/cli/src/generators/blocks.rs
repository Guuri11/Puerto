//! `puerto generar web-bloque` (ADR-0028): a block is a variant of a slot, a file of `web/` with a
//! fixed export and props, installed over the one `puerto nuevo` wrote. Every variant keeps its
//! slot's contract and the texts the template's tests look for, so installing one touches nothing
//! else: no generated file, no test, not `puerto.toml`.

use crate::generators::conflict::{OverwritePolicy, guard_files_overwrite};
use crate::generators::layout;
use crate::generators::naming::write_file;
use crate::generators::render::render;
use std::fs;
use std::path::{Path, PathBuf};

/// What a block asks of the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Needs {
    Nothing,
    /// `[proyecto] auth`: the slot exists only with auth.
    Auth,
    /// `[proyecto] portada`: the slot is a page of the portada.
    Portada,
    /// A portada made of sections (`portada/producto`): the slot is one of them.
    Sections,
}

/// Where a block file's content comes from.
#[derive(Debug, Clone, Copy)]
enum Source {
    /// The template's own file (`template/web/<file>`): what `puerto nuevo` writes.
    Base,
    /// With auth, what auth writes over the template's file (`templates/auth/web/<file>.tera`).
    BaseOrAuth,
    /// What auth writes (`templates/auth/web/<file>.tera`): the slot exists only with auth.
    Auth,
    /// A variant of its own: `templates/web/bloques/<slot>/<variant>/<file name>.tera`.
    Variant,
    /// Another block's file: a page brings the default variant of each of its sections.
    Block(&'static str),
}

pub struct Block {
    /// `<slot>/<variant>`.
    pub id: &'static str,
    pub description: &'static str,
    pub needs: Needs,
    /// Each file, relative to `web/`, and where its content comes from.
    files: &'static [(&'static str, Source)],
}

const MARCO_ACCESO: &str = "src/compartido/ui/marco-acceso.tsx";
const MARCO: &str = "src/compartido/ui/marco.tsx";
const PANEL: &str = "src/paginas/panel.tsx";
const PERFIL: &str = "src/paginas/perfil.tsx";
const TEMA: &str = "src/marca/tema.css";
const PORTADA: &str = "src/paginas/portada.tsx";
const CABECERA: &str = "src/portada/cabecera.tsx";
const CARACTERISTICAS: &str = "src/portada/caracteristicas.tsx";
const PRECIOS: &str = "src/portada/precios.tsx";
const PREGUNTAS: &str = "src/portada/preguntas.tsx";
const LLAMADA: &str = "src/portada/llamada.tsx";
const PIE: &str = "src/portada/pie.tsx";

const fn block(
    id: &'static str,
    description: &'static str,
    needs: Needs,
    files: &'static [(&'static str, Source)],
) -> Block {
    Block {
        id,
        description,
        needs,
        files,
    }
}

/// Every block, slot by slot; the first variant of a slot is the one `puerto nuevo` writes.
pub const CATALOG: &[Block] = &[
    block(
        "acceso/dividido",
        "la marca sobre el color primario a un lado, el formulario en una tarjeta al otro",
        Needs::Auth,
        &[(MARCO_ACCESO, Source::Auth)],
    ),
    block(
        "acceso/centrado",
        "el formulario en una tarjeta en el centro, la marca encima y el eslogan debajo",
        Needs::Auth,
        &[(MARCO_ACCESO, Source::Variant)],
    ),
    block(
        "marco/lateral",
        "la marca y el menú en un lateral; en el móvil, tras un botón",
        Needs::Nothing,
        &[(MARCO, Source::BaseOrAuth)],
    ),
    block(
        "marco/superior",
        "la marca y el menú en una barra arriba; en el móvil, un desplegable",
        Needs::Nothing,
        &[(MARCO, Source::Variant)],
    ),
    block(
        "panel/tarjetas",
        "una tarjeta por sección con su total",
        Needs::Nothing,
        &[(PANEL, Source::Base)],
    ),
    block(
        "panel/resumen",
        "una banda de color con el título y los totales en una fila",
        Needs::Nothing,
        &[(PANEL, Source::Variant)],
    ),
    block(
        "perfil/tarjeta",
        "el avatar, el nombre, el email y los roles en una tarjeta",
        Needs::Auth,
        &[(PERFIL, Source::Auth)],
    ),
    block(
        "perfil/banda",
        "una banda de color con el avatar montado encima",
        Needs::Auth,
        &[(PERFIL, Source::Variant)],
    ),
    block(
        "portada/bienvenida",
        "el logo, el nombre, el eslogan y «Entrar»",
        Needs::Portada,
        &[(PORTADA, Source::Variant)],
    ),
    block(
        "portada/producto",
        "una web de producto: barra, cabecera, características, precios, preguntas, llamada y pie",
        Needs::Portada,
        &[
            (PORTADA, Source::Variant),
            (CABECERA, Source::Block("cabecera/centrada")),
            (CARACTERISTICAS, Source::Block("caracteristicas/rejilla")),
            (PRECIOS, Source::Block("precios/planes")),
            (PREGUNTAS, Source::Block("preguntas/acordeon")),
            (LLAMADA, Source::Block("llamada/banda")),
            (PIE, Source::Block("pie/columnas")),
        ],
    ),
    block(
        "cabecera/centrada",
        "el eslogan en grande en el centro y la entrada a la app",
        Needs::Sections,
        &[(CABECERA, Source::Variant)],
    ),
    block(
        "cabecera/dividida",
        "el eslogan a un lado y, al otro, un navegador con un panel de ejemplo",
        Needs::Sections,
        &[(CABECERA, Source::Variant)],
    ),
    block(
        "caracteristicas/rejilla",
        "seis características con su icono, en una rejilla",
        Needs::Sections,
        &[(CARACTERISTICAS, Source::Variant)],
    ),
    block(
        "precios/planes",
        "tres planes, el del centro destacado",
        Needs::Sections,
        &[(PRECIOS, Source::Variant)],
    ),
    block(
        "preguntas/acordeon",
        "las preguntas frecuentes en un acordeón",
        Needs::Sections,
        &[(PREGUNTAS, Source::Variant)],
    ),
    block(
        "llamada/banda",
        "una banda del color de la marca que invita a entrar",
        Needs::Sections,
        &[(LLAMADA, Source::Variant)],
    ),
    block(
        "pie/columnas",
        "la marca y los enlaces de la portada, en columnas",
        Needs::Sections,
        &[(PIE, Source::Variant)],
    ),
    block(
        "tema/puerto",
        "cian y ámbar, esquinas redondeadas",
        Needs::Nothing,
        &[(TEMA, Source::Base)],
    ),
    block(
        "tema/bosque",
        "verde y lima",
        Needs::Nothing,
        &[(TEMA, Source::Variant)],
    ),
    block(
        "tema/ciruela",
        "morado y rosa, muy redondeado",
        Needs::Nothing,
        &[(TEMA, Source::Variant)],
    ),
    block(
        "tema/coral",
        "coral y turquesa, botones de píldora",
        Needs::Nothing,
        &[(TEMA, Source::Variant)],
    ),
    block(
        "tema/pizarra",
        "casi negro y azul, esquinas rectas",
        Needs::Nothing,
        &[(TEMA, Source::Variant)],
    ),
    block(
        "tema/miel",
        "ámbar y marrón sobre un fondo cálido",
        Needs::Nothing,
        &[(TEMA, Source::Variant)],
    ),
];

fn slot(id: &str) -> &str {
    id.split_once('/').map_or(id, |(slot, _)| slot)
}

/// The catalog as `puerto generar web-bloque` prints it without an argument.
pub fn catalog_text() -> String {
    let width = CATALOG.iter().map(|b| b.id.len()).max().unwrap_or(0);
    let mut text = String::new();
    let mut previous = "";
    for block in CATALOG {
        if !previous.is_empty() && slot(block.id) != previous {
            text.push('\n');
        }
        previous = slot(block.id);
        let needs = match block.needs {
            Needs::Nothing => "",
            Needs::Auth => " (auth)",
            Needs::Portada => " (portada)",
            Needs::Sections => " (portada/producto)",
        };
        text.push_str(&format!(
            "  {:width$}  {}{needs}\n",
            block.id, block.description
        ));
    }
    text
}

fn find(id: &str) -> Result<&'static Block, String> {
    if let Some(block) = CATALOG.iter().find(|b| b.id == id) {
        return Ok(block);
    }
    let variants: Vec<&str> = CATALOG
        .iter()
        .map(|b| b.id)
        .filter(|b| slot(b) == slot(id))
        .collect();
    if !variants.is_empty() {
        return Err(format!(
            "'{id}' no es un bloque. Las variantes de {} son: {}",
            slot(id),
            variants.join(", ")
        ));
    }
    let mut slots: Vec<&str> = CATALOG.iter().map(|b| slot(b.id)).collect();
    slots.dedup();
    Err(format!(
        "'{id}' no es un bloque. Los huecos son: {}. Ejecuta `puerto generar web-bloque` para ver el catálogo",
        slots.join(", ")
    ))
}

/// The block's files, relative to the project root, as they would be written in a project whose
/// auth mode is `auth`.
fn render_block(
    block: &Block,
    auth: Option<&str>,
) -> Result<Vec<(PathBuf, String)>, Box<dyn std::error::Error>> {
    let mut ctx = tera::Context::new();
    ctx.insert("auth", &auth.is_some());
    ctx.insert("local", &(auth == Some("local")));
    block
        .files
        .iter()
        .map(|(file, source)| {
            let content = match (source, auth) {
                (Source::Base, _) | (Source::BaseOrAuth, None) => base_file(file)?,
                (Source::BaseOrAuth, Some(_)) | (Source::Auth, _) => {
                    render(&format!("auth/web/{file}.tera"), &ctx)?
                }
                (Source::Variant, _) => render(&variant_template(block.id, file)?, &ctx)?,
                (Source::Block(id), _) => render(&variant_template(id, file)?, &ctx)?,
            };
            Ok((Path::new(layout::WEB_DIR).join(file), content))
        })
        .collect()
}

/// `templates/web/bloques/<slot>/<variant>/<file name>.tera`.
fn variant_template(id: &str, file: &str) -> Result<String, String> {
    let name = Path::new(file).file_name().and_then(|n| n.to_str());
    let name = name.ok_or_else(|| format!("{file}: sin nombre de fichero"))?;
    Ok(format!("web/bloques/{id}/{name}.tera"))
}

fn base_file(file: &str) -> Result<String, String> {
    let path = format!("web/{file}");
    crate::TEMPLATE_DIR
        .get_file(&path)
        .and_then(|f| f.contents_utf8())
        .map(str::to_string)
        .ok_or_else(|| format!("la plantilla no tiene {path}"))
}

/// The project's portada mounts the sections of `web/src/portada/` (`portada/producto`).
fn composes_sections(base: &Path) -> bool {
    fs::read_to_string(base.join(layout::WEB_DIR).join(PORTADA))
        .is_ok_and(|page| page.contains("from \"../portada/"))
}

/// What installing a block did.
#[derive(Debug)]
pub struct Installed {
    /// Every file of the block, relative to the project root.
    pub files: Vec<PathBuf>,
    /// The ones that changed: none when the block was already installed.
    pub written: Vec<PathBuf>,
}

/// Installs the block `id` in the project at `base`. A file with other content is protected
/// unless `policy` forces it.
pub fn run_install(
    id: &str,
    base: &Path,
    policy: OverwritePolicy,
) -> Result<Installed, Box<dyn std::error::Error>> {
    let block = find(id)?;
    let config = crate::puerto_toml::read(base)?;
    if !config.project.web {
        return Err(
            "este proyecto no tiene frontend ([proyecto] web no es true en puerto.toml)".into(),
        );
    }
    let auth = config.project.auth.as_deref();
    if block.needs == Needs::Auth && auth.is_none() {
        return Err(format!(
            "'{id}' es una pantalla de auth, y este proyecto no tiene auth ([proyecto] auth en puerto.toml)"
        )
        .into());
    }
    if matches!(block.needs, Needs::Portada | Needs::Sections) && !config.project.portada {
        return Err(format!(
            "'{id}' es de la portada, y este proyecto no tiene ([proyecto] portada: `puerto nuevo --portada`)"
        )
        .into());
    }
    if block.needs == Needs::Sections && !composes_sections(base) {
        return Err(format!(
            "'{id}' es una sección de portada/producto, y la portada de este proyecto no tiene \
             secciones: instala antes `puerto generar web-bloque portada/producto`"
        )
        .into());
    }

    let rendered = render_block(block, auth)?;
    let files: Vec<PathBuf> = rendered.iter().map(|(path, _)| path.clone()).collect();
    let changed: Vec<(PathBuf, String)> = rendered
        .into_iter()
        .filter(|(path, content)| {
            fs::read_to_string(base.join(path)).ok().as_ref() != Some(content)
        })
        .collect();
    let written: Vec<PathBuf> = changed.iter().map(|(path, _)| path.clone()).collect();
    guard_files_overwrite(base, id, &written, policy)?;
    for (path, content) in &changed {
        write_file(&base.join(path), content)?;
    }
    Ok(Installed { files, written })
}
