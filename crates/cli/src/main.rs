use cargo_generate::{GenerateArgs, TemplatePath, generate};
use std::path::PathBuf;

fn templates_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.push("../template");
    dir
}

fn run() -> Result<String, Box<dyn std::error::Error>> {
    let template_dir = templates_dir().join("basic");

    let args = GenerateArgs {
        template_path: TemplatePath {
            path: Some(template_dir.to_string_lossy().into_owned()),
            ..Default::default()
        },
        no_workspace: true,
        ..Default::default()
    };

    let output_dir = generate(args)?;
    Ok(output_dir.display().to_string())
}

fn main() {
    match run() {
        Ok(path) => println!("Project created at: {path}"),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_generate::Vcs;
    use std::fs;
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("harbor_test_{name}"))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    fn generate_project(
        name: &str,
        destination: &Path,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let template_dir = templates_dir().join("basic");

        let args = GenerateArgs {
            template_path: TemplatePath {
                path: Some(template_dir.to_string_lossy().into_owned()),
                ..Default::default()
            },
            name: Some(name.to_string()),
            destination: Some(destination.to_path_buf()),
            vcs: Some(Vcs::None),
            no_workspace: true,
            ..Default::default()
        };

        let output_dir = generate(args)?;
        Ok(output_dir)
    }

    #[test]
    fn creates_project_structure() {
        let dir = temp_dir("cg_structure");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("test-app", &dir).unwrap();

        // Workspace layout
        assert!(output.join("Cargo.toml").exists());
        assert!(output.join("business/src/lib.rs").exists());
        assert!(output.join("infrastructure/src/lib.rs").exists());
        assert!(output.join("presentation/src/main.rs").exists());

        cleanup(&dir);
    }

    #[test]
    fn presentation_cargo_toml_has_project_name() {
        let dir = temp_dir("cg_cargo");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("my-harbor-app", &dir).unwrap();

        let content = fs::read_to_string(output.join("presentation/Cargo.toml")).unwrap();
        assert!(content.contains("name = \"my-harbor-app\""));

        cleanup(&dir);
    }

    #[test]
    fn main_rs_wires_ddd_layers() {
        let dir = temp_dir("cg_main");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("cool-project", &dir).unwrap();

        let content = fs::read_to_string(output.join("presentation/src/main.rs")).unwrap();
        assert!(content.contains("GetGreetingUseCaseImpl"));
        assert!(content.contains("InMemoryGreetingRepository"));
        assert!(content.contains("GreetingApi"));
        assert!(!content.contains("{{project-name}}"));

        cleanup(&dir);
    }

    #[test]
    fn main_rs_has_poem_openapi() {
        let dir = temp_dir("cg_poem");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("api-test", &dir).unwrap();

        let content = fs::read_to_string(output.join("presentation/src/main.rs")).unwrap();
        assert!(content.contains("OpenApiService"));
        assert!(content.contains("8080"));

        cleanup(&dir);
    }

    #[test]
    #[ignore = "slow: compiles and tests a full generated project"]
    fn generated_project_compiles_and_tests_pass() {
        let dir = temp_dir("cg_compile");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("compile-test", &dir).unwrap();

        let result = std::process::Command::new("cargo")
            .args(["test", "--workspace"])
            .current_dir(&output)
            .output()
            .expect("failed to run cargo test");

        if !result.status.success() {
            eprintln!("stdout:\n{}", String::from_utf8_lossy(&result.stdout));
            eprintln!("stderr:\n{}", String::from_utf8_lossy(&result.stderr));
            panic!("cargo test failed in generated project");
        }

        cleanup(&dir);
    }

    #[test]
    fn ddd_layers_exist() {
        let dir = temp_dir("cg_ddd");
        cleanup(&dir);
        fs::create_dir_all(&dir).unwrap();

        let output = generate_project("ddd-app", &dir).unwrap();

        // Domain
        assert!(output.join("business/src/domain/greeting/model.rs").exists());
        assert!(output.join("business/src/domain/greeting/errors.rs").exists());
        assert!(output.join("business/src/domain/greeting/repository.rs").exists());
        assert!(output.join("business/src/domain/greeting/use_cases/get_greeting.rs").exists());
        // Application
        assert!(output.join("business/src/application/greeting/get_greeting.rs").exists());
        // Infrastructure
        assert!(output.join("infrastructure/src/greeting/repository.rs").exists());
        // Presentation
        assert!(output.join("presentation/src/api/greeting/routes.rs").exists());
        assert!(output.join("presentation/src/api/greeting/dto.rs").exists());
        assert!(output.join("presentation/src/api/greeting/error_mapper.rs").exists());
        assert!(output.join("presentation/src/api/greeting/responses.rs").exists());

        cleanup(&dir);
    }
}
