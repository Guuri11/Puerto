use include_dir::{Dir, include_dir};
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
    let content = TEMPLATES
        .get_file(template_name)
        .ok_or_else(|| tera::Error::msg(format!("template not found: {template_name}")))?
        .contents_utf8()
        .ok_or_else(|| tera::Error::msg("template not UTF-8"))?;
    let mut tera = Tera::default();
    tera.add_raw_template(template_name, content)?;
    tera.render(template_name, ctx)
}
