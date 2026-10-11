use include_dir::{Dir, include_dir};
use std::sync::OnceLock;
use tera::Tera;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/templates");

/// A template that failed to render. Its message names the template and the whole tera cause
/// chain — `tera::Error`'s own `Display` stops at "Failed to render '…'" and hides the reason.
#[derive(Debug)]
pub struct RenderError {
    template: String,
    cause: String,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cannot render template {}: {}",
            self.template, self.cause
        )
    }
}

impl std::error::Error for RenderError {}

pub fn render(template_name: &str, ctx: &tera::Context) -> Result<String, RenderError> {
    render_tera(template_name, ctx).map_err(|e| {
        let mut cause = e.to_string();
        let mut source = std::error::Error::source(&e);
        while let Some(s) = source {
            cause.push_str(": ");
            cause.push_str(&s.to_string());
            source = s.source();
        }
        RenderError {
            template: template_name.to_string(),
            cause,
        }
    })
}

fn render_tera(template_name: &str, ctx: &tera::Context) -> Result<String, tera::Error> {
    match templates() {
        Ok(tera) => tera.render(template_name, ctx),
        Err(e) => Err(tera::Error::msg(e)),
    }
}

/// Every embedded template, parsed once. One `Tera` instance holds them all so a template can
/// `{% import %}` macros from another (`domain/value_object_macros.tera`).
fn templates() -> &'static Result<Tera, String> {
    static TERA: OnceLock<Result<Tera, String>> = OnceLock::new();
    TERA.get_or_init(|| {
        let mut files = vec![];
        collect(&TEMPLATES, &mut files)?;
        let mut tera = Tera::default();
        tera.add_raw_templates(files).map_err(|e| {
            let mut msg = e.to_string();
            let mut source = std::error::Error::source(&e);
            while let Some(s) = source {
                msg.push_str(&format!(": {s}"));
                source = s.source();
            }
            msg
        })?;
        Ok(tera)
    })
}

fn collect(
    dir: &'static Dir<'static>,
    out: &mut Vec<(String, &'static str)>,
) -> Result<(), String> {
    for file in dir.files() {
        let name = file.path().to_string_lossy().into_owned();
        let content = file
            .contents_utf8()
            .ok_or_else(|| format!("template {name} is not UTF-8"))?;
        out.push((name, content));
    }
    for sub in dir.dirs() {
        collect(sub, out)?;
    }
    Ok(())
}
