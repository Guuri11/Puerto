//! A mail's templates: `subject.txt`, `body.html` and the optional `body.txt`, embedded by the
//! generated code with `include_str!` and rendered with minijinja.

use puerto_core::mail::{Email, MailError};
use serde::Serialize;

/// One mail's three templates. Built as a `const` in the generated code:
///
/// ```
/// use puerto_mail::Templates;
///
/// const WELCOME: Templates = Templates::new("Welcome, {{ name }}", "<p>Hi {{ name }}</p>")
///     .with_text("Hi {{ name }}");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Templates {
    pub subject: &'static str,
    pub html: &'static str,
    pub text: Option<&'static str>,
}

impl Templates {
    pub const fn new(subject: &'static str, html: &'static str) -> Self {
        Self {
            subject,
            html,
            text: None,
        }
    }

    pub const fn with_text(mut self, text: &'static str) -> Self {
        self.text = Some(text);
        self
    }

    /// The message to `to`, rendered from `context` (the mail's fields).
    ///
    /// Strict: a variable the context does not have is an error, never an empty string. The HTML
    /// body escapes what it interpolates; the subject is folded onto one line. Any failure is a
    /// [`MailError::Permanent`] — rendering the same event again cannot fix a template.
    pub fn render(
        &self,
        to: impl Into<String>,
        context: impl Serialize,
    ) -> Result<Email, MailError> {
        let mut env = minijinja::Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
        let context = minijinja::Value::from(minijinja::value::Serde(&context));
        let render = |name: &'static str, source: &'static str| {
            env.render_named_str(name, source, &context)
                .map_err(|e| MailError::Permanent(format!("{name}: {e}")))
        };

        let subject = render("subject.txt", self.subject)?
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let html = render("body.html", self.html)?;
        let email = Email::new(to, subject, html);
        Ok(match self.text {
            Some(text) => email.with_text(render("body.txt", text)?),
            None => email,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Welcome {
        name: String,
    }

    fn ada() -> Welcome {
        Welcome { name: "Ada".into() }
    }

    const WELCOME: Templates =
        Templates::new("Welcome, {{ name }}", "<p>Hi {{ name }}</p>").with_text("Hi {{ name }}");

    #[test]
    fn should_render_every_part_when_the_context_has_its_variables() {
        // Act
        let email = WELCOME.render("ada@example.com", ada());

        // Assert
        assert_eq!(
            email,
            Ok(Email::new("ada@example.com", "Welcome, Ada", "<p>Hi Ada</p>").with_text("Hi Ada"))
        );
    }

    #[test]
    fn should_send_html_only_when_there_is_no_text_template() {
        let templates = Templates::new("Hi", "<p>{{ name }}</p>");

        let email = templates.render("ada@example.com", ada());

        assert_eq!(email.map(|e| e.text), Ok(None));
    }

    #[test]
    fn should_fail_permanently_when_a_template_names_an_unknown_variable() {
        // Arrange
        let templates = Templates::new("Hi {{ nmae }}", "<p>Hi</p>");

        // Act
        let result = templates.render("ada@example.com", ada());

        // Assert
        match result {
            Err(MailError::Permanent(detail)) => {
                assert!(detail.starts_with("subject.txt"), "{detail}")
            }
            other => panic!("expected a permanent error, got {other:?}"),
        }
    }

    #[test]
    fn should_fail_permanently_when_a_template_does_not_parse() {
        let templates = Templates::new("Hi", "<p>{% if name %}</p>");

        let result = templates.render("ada@example.com", ada());

        assert!(matches!(result, Err(MailError::Permanent(_))), "{result:?}");
    }

    #[test]
    fn should_fold_the_subject_onto_one_line_when_it_spans_several() {
        let templates = Templates::new("Welcome,\n  {{ name }}\r\n", "<p></p>");

        let email = templates.render("ada@example.com", ada());

        assert_eq!(email.map(|e| e.subject), Ok("Welcome, Ada".to_string()));
    }

    #[test]
    fn should_escape_html_when_a_value_holds_markup() {
        let templates = Templates::new("{{ name }}", "<p>{{ name }}</p>").with_text("{{ name }}");
        let context = Welcome {
            name: "<b>Ada</b>".into(),
        };

        let email = templates.render("ada@example.com", context).unwrap();

        assert_eq!(email.html, "<p>&lt;b&gt;Ada&lt;&#x2f;b&gt;</p>");
        assert_eq!(email.subject, "<b>Ada</b>");
        assert_eq!(email.text.as_deref(), Some("<b>Ada</b>"));
    }
}
