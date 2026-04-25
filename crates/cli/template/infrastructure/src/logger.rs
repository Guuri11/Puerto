use business::domain::logger::LoggerTrait;

pub struct TracingLogger;

impl LoggerTrait for TracingLogger {
    fn info(&self, message: &str) {
        tracing::info!("{}", message);
    }
    fn warn(&self, message: &str) {
        tracing::warn!("{}", message);
    }
    fn error(&self, message: &str) {
        tracing::error!("{}", message);
    }
    fn debug(&self, message: &str) {
        tracing::debug!("{}", message);
    }
}
