use super::errors::GreetingError;

/// A greeting message for a given name
#[derive(Debug, Clone)]
pub struct Greeting {
    pub name: String,
    pub message: String,
}

impl Greeting {
    pub fn new(name: &str) -> Result<Self, GreetingError> {
        if name.trim().is_empty() {
            return Err(GreetingError::ValidationError("name_empty".into()));
        }

        Ok(Self {
            name: name.to_string(),
            message: format!("Hello, {}! Greetings from Harbor.", name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_greeting_with_valid_name() {
        let result = Greeting::new("World");
        assert!(result.is_ok());
        let greeting = result.unwrap();
        assert_eq!(greeting.name, "World");
        assert!(greeting.message.contains("World"));
    }

    #[test]
    fn should_fail_when_name_is_empty() {
        let result = Greeting::new("");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "greeting.validation_error.name_empty"
        );
    }

    #[test]
    fn should_fail_when_name_is_only_whitespace() {
        let result = Greeting::new("   ");
        assert!(result.is_err());
    }
}
