pub mod domain {
  pub mod greeting {
    pub mod errors;
    pub mod model;
    pub mod repository;
    pub mod use_cases;
  }
}
pub mod application {
  pub mod greeting {
    pub mod get_greeting;
  }
}
