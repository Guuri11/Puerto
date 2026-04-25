pub mod domain {
  pub mod logger;
  pub mod greeting {
    pub mod errors;
    pub mod model;
    pub mod repository;
    pub mod use_cases {
      pub mod get_greeting;
    }
  }
}
pub mod application {
  pub mod greeting {
    pub mod get_greeting;
  }
}
