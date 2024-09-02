use std::{fmt, future::ready, net::SocketAddr, str::FromStr, time::{Duration, SystemTime}};

use axum::{async_trait, body::Body, error_handling::HandleErrorLayer, http::{Request, Response, HeaderMap, StatusCode, Uri}, middleware, routing::get, BoxError, Json, Router};
use bytes::Bytes;
use fred::prelude::RedisPool;
use sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;
use serde::{de, Deserialize, Deserializer};
use serde_json::json;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{classify::ServerErrorsFailureClass, cors::{Any, CorsLayer}, request_id::{MakeRequestUuid, RequestId}, trace::TraceLayer, ServiceBuilderExt};
use tracing::{debug, info, Span};

use crate::utils::error::SystemErrorCode;

#[derive(Clone)]
pub struct AppState {
    pub app_name: &'static str,
    // A database connection used by the application.
    pub db: DatabaseConnection,
    // A redis pool used by the application.
    pub redis: RedisPool,
    pub jwt_secret_access: String,
    pub jwt_secret_refresh: String,
    // Configuration settings for the application
    // pub config: Config,
    // An optional email sender component that can be used to send email.
    // pub mailer: Option<EmailSender>,
    // An optional storage instance for the application
    // pub storage: Arc<Storage>,
    // Cache instance for the application
    // pub cache: Arc<cache::Cache>,
}

#[async_trait]
pub trait Hooks {
    fn app_name() -> &'static str;
    fn router(state: AppState) -> Router;
    fn clean_up();
}

pub async fn main<H: Hooks, M: MigratorTrait>() {
    let init_date_time = SystemTime::now();
    // 初始化环境变量
    dotenvy::dotenv().ok();
    // for (key, value) in std::env::vars() {
    //     println!("{key}: {value}");
    // }
    // 初始化日志
    let (_guard_stdout, _guard_file) = crate::utils::log::init("./logs", "admin-api.log");

    let db = crate::utils::database::init::<M>().await;
    let redis = crate::utils::redis::init().await;
    let jwt_secret_access = std::env::var("JWT_SECRET_ACCESS").expect("JWT_SECRET_ACCESS is not set in .env file");
    let jwt_secret_refresh = std::env::var("JWT_SECRET_REFRESH").expect("JWT_SECRET_REFRESH is not set in .env file");

    let state = AppState { app_name: H::app_name(), db, redis, jwt_secret_access, jwt_secret_refresh };
    
    let (_main_server, _metrics_server) = tokio::join!(start_main_server(H::router(state.clone()), state), start_metrics_server());
    info!("开始清理资源");
    H::clean_up();
    info!("清理资源完成");
    // endregion: --- Start Server
    info!("本次运行时间: {}", Wrapper(SystemTime::now().duration_since(init_date_time).unwrap_or_default()));
}

async fn start_main_server(router: Router, state: AppState) {
    let app = main_app(router, state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
    .with_graceful_shutdown(shutdown_signal()).await.unwrap();
}

fn main_app(router: Router, _state: AppState) -> Router {
    // -- Cors
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any)
        // .max_age(Duration::from_secs(60) * 10)
    ;
    
    let trace_layer = ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid::default())
        .layer(TraceLayer::new_for_http()
            .make_span_with(|request: &Request<Body>| {
                let id = request.extensions().get::<RequestId>().unwrap().header_value().to_str().unwrap();
                tracing::info_span!("request", %id)
            })
            .on_request(|request: &Request<Body>, _span: &Span| {
                tracing::debug!("started {} {}", request.method(), request.uri().path())
            })
            .on_response(|_response: &Response<Body>, latency: Duration, _span: &Span| {
                tracing::debug!("response generated in {:?}", latency)
            })
            .on_body_chunk(|chunk: &Bytes, _latency: Duration, _span: &Span| {
                tracing::debug!("sending {} bytes: {}", chunk.len(), String::from_utf8(chunk.to_vec()).unwrap_or_default())
            })
            .on_eos(|_trailers: Option<&HeaderMap>, stream_duration: Duration, _span: &Span| {
                tracing::debug!("stream closed after {:?}", stream_duration)
            })
            .on_failure(|error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                tracing::debug!("{}", error)
            })
        )
        .propagate_x_request_id()
    ;

    Router::new().merge(router)
    .layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_timeout_error))
            .load_shed()
            .concurrency_limit(1024)
            // .rate_limit(100, Duration::from_secs(1))
            .timeout(Duration::from_secs(60))
            .layer(TraceLayer::new_for_http()),
    )
    .layer(cors)
    .layer(trace_layer)
    .layer(middleware::from_fn(crate::utils::prometheus::track_metrics))
    .fallback(json_fallback)
}

fn metrics_app() -> Router {
    let recorder_handle = crate::utils::prometheus::setup_metrics_recorder();
    Router::new().route("/metrics", get(move || ready(recorder_handle.render())))
}

async fn start_metrics_server() {
    let app = metrics_app();

    // NOTE: expose metrics endpoint on a different port
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
    .with_graceful_shutdown(shutdown_signal()).await.unwrap();
}

///
/// 关闭程序
///
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("未能侦听ctrl-c事件");
    };

    #[cfg(unix)]
        let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("无法安装信号处理程序")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    // info!("开始优雅停机");
}

///
/// 处理超时
///
async fn handle_timeout_error(uri: Uri, error: BoxError) -> (StatusCode, Json<serde_json::Value>) {
    if error.is::<tower::timeout::error::Elapsed>() {
        debug!("请求超时: {}", uri.path());
        let error_result = Json(json!({
            "status": false,
            "error_code": SystemErrorCode::RequestTimeout as usize,
            "error_message": "请求超时",
        }));
        (StatusCode::REQUEST_TIMEOUT, error_result)
    } else if error.is::<tower::load_shed::error::Overloaded>() {
        debug!("服务过载: {}", uri.path());
        let error_result = Json(json!({
            "status": false,
            "error_code": SystemErrorCode::ServiceUnavailable as usize,
            "error_message": "服务过载，请稍后再试",
        }));
        (StatusCode::SERVICE_UNAVAILABLE, error_result)
    } else {
        debug!("未处理的内部错误: {}, {}", uri.path(), error);
        let error_result = Json(json!({
            "status": false,
            "error_code": SystemErrorCode::InternalServerError as usize,
            "error_message": "未处理的内部错误",
        }));
        (StatusCode::INTERNAL_SERVER_ERROR, error_result)
    }
}

///
/// 找不到路由地址
///
async fn json_fallback(uri: Uri) -> (StatusCode, Json<serde_json::Value>) {
    let msg = format!("找不到路由: {}", uri.path());
    debug!("{}", &msg);
    let error_result = Json(json!({
        "status": false,
        "error_code": SystemErrorCode::NotFound as usize,
        "error_message": msg,
    }));
    (StatusCode::NOT_FOUND, error_result)
}


/// Serde deserialization decorator to map empty Strings to None,
#[allow(unused)]
fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

struct Wrapper(Duration);

impl std::fmt::Display for Wrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let duration = time::Duration::try_from(self.0).unwrap_or_default();
        let d = duration.whole_days();
        let h = duration.whole_hours() % 24;
        let m = duration.whole_minutes() % 60;
        let s = duration.whole_seconds() % 60;
        let ms = duration.whole_milliseconds() % 1000;

        if duration.whole_seconds() > 0 {
            if duration.whole_minutes() > 0 {
                if duration.whole_hours() > 0 {
                    if duration.whole_days() > 0 {
                        write!(f, "{}天", d)?;
                    }
                    write!(f, "{}小时", h)?;
                }
                write!(f, "{}分钟", m)?;
            }
            write!(f, "{}秒", s)?;
        }
        write!(f, "{}毫秒", ms)?;
        Ok(())
    }
}
