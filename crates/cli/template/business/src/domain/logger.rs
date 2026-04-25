use std::sync::Arc;

pub trait LoggerTrait: Send + Sync {
    fn info(&self, message: &str);
    fn warn(&self, message: &str);
    fn error(&self, message: &str);
    fn debug(&self, message: &str);
}

pub fn noop() -> Arc<dyn LoggerTrait> {
    struct NoopLogger;
    impl LoggerTrait for NoopLogger {
        fn info(&self, _: &str) {}
        fn warn(&self, _: &str) {}
        fn error(&self, _: &str) {}
        fn debug(&self, _: &str) {}
    }
    Arc::new(NoopLogger)
}

#[cfg(any(test, feature = "test-utils"))]
pub mod mocks {
    use mockall::mock;
    use super::LoggerTrait;

    mock! {
        pub Logger {}
        impl LoggerTrait for Logger {
            fn info(&self, message: &str);
            fn warn(&self, message: &str);
            fn error(&self, message: &str);
            fn debug(&self, message: &str);
        }
    }
}
