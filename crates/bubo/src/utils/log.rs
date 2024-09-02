use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::non_blocking::{NonBlockingBuilder, WorkerGuard};

pub fn init(dir: &str, file: &str) -> (WorkerGuard, WorkerGuard) {
    // 控制台非阻塞不丢失日志配置
    let (non_blocking_stdout, guard_stdout) = NonBlockingBuilder::default().lossy(false).finish(std::io::stdout());
    let layer_stdout = tracing_subscriber::fmt::layer().with_ansi(true).with_writer(non_blocking_stdout);
    // 文件非阻塞不丢失日志配置
    let file_appender = tracing_appender::rolling::daily(dir, file);
    let (non_blocking_file, guard_file) = NonBlockingBuilder::default().lossy(false).finish(file_appender);
    let layer_file = tracing_subscriber::fmt::layer().with_ansi(false).with_writer(non_blocking_file);

    // 日志订阅
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(layer_stdout)
        .with(layer_file)
        .init();
    info!("日志初始化完成");
    (guard_stdout, guard_file)
}