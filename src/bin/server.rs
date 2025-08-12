use axum::extract::DefaultBodyLimit;
use axum::http::Method;
use axum::{
    extract::State,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use chai::config::{ObjectiveConfig, 配置};
use chai::interfaces::server::WebApi;
use chai::interfaces::{默认输入, 消息};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::timeout::TimeoutLayer;
use tracing::info;

/// HTTP API 响应类型
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiResponse<T> {
    #[serde(rename = "success")]
    Success { result: T },
    #[serde(rename = "error")]
    Error { error: String },
}

/// 应用状态
#[derive(Clone)]
pub struct AppState {
    /// 全局 WebApi 实例
    pub api: Arc<Mutex<WebApi>>,
    /// 优化状态
    pub optimization_status: Arc<Mutex<OptimizationStatus>>,
}

/// 优化状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStatus {
    pub is_running: bool,
    pub progress: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// HTTP API: 验证配置
pub async fn validate_config(Json(config): Json<serde_json::Value>) -> Json<ApiResponse<配置>> {
    info!("POST /api/validate");

    // 直接在服务器中验证配置
    match serde_json::from_value::<配置>(config) {
        Ok(config) => {
            // 配置解析成功，可以在这里添加额外的验证逻辑
            // 例如：检查必填字段、验证数值范围等
            Json(ApiResponse::Success { result: config })
        }
        Err(e) => {
            // 配置解析失败
            Json(ApiResponse::Error {
                error: format!("配置解析错误: {e}"),
            })
        }
    }
}

/// HTTP API: 同步参数
pub async fn sync_params(
    State(state): State<AppState>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<()>> {
    info!("POST /api/sync");

    // 直接转换为图形界面参数
    match serde_json::from_value::<默认输入>(params) {
        Ok(图形界面参数) => {
            let result = {
                let mut api = state.api.lock().unwrap();
                api.sync(图形界面参数)
            }; // 锁在这里被释放

            match result {
                Ok(_) => Json(ApiResponse::Success { result: () }),
                Err(e) => Json(ApiResponse::Error { error: e.message }),
            }
        }
        Err(e) => Json(ApiResponse::Error {
            error: format!("参数解析错误: {e}"),
        }),
    }
}

/// HTTP API: 编码评估
pub async fn encode_evaluate(
    State(state): State<AppState>,
    Json(objective): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    info!("POST /api/encode");

    // 直接转换为目标函数配置
    match serde_json::from_value::<ObjectiveConfig>(objective) {
        Ok(目标函数配置) => {
            let result = {
                let api = state.api.lock().unwrap();
                api.encode_evaluate(目标函数配置)
            }; // 锁在这里被释放

            match result {
                Ok(result) => Json(ApiResponse::Success {
                    result: serde_json::json!([result.0, result.1]),
                }),
                Err(e) => Json(ApiResponse::Error { error: e.message }),
            }
        }
        Err(e) => Json(ApiResponse::Error {
            error: format!("目标函数配置解析错误: {e}"),
        }),
    }
}

/// HTTP API: 开始优化（异步）
pub async fn start_optimize(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    info!("POST /api/optimize");

    // 检查是否已经在运行
    {
        let status = state.optimization_status.lock().unwrap();
        if status.is_running {
            return Json(ApiResponse::Error {
                error: "优化已在进行中".to_string(),
            });
        }
    }

    // 设置开始状态
    {
        let mut status = state.optimization_status.lock().unwrap();
        status.is_running = true;
        status.progress = None;
        status.error = None;
    }

    let api = state.api.clone();
    let status = state.optimization_status.clone();

    // 在后台启动优化任务
    tokio::spawn(async move {
        let result = {
            let api_guard = api.lock().unwrap();
            api_guard.optimize()
        }; // 锁在这里被释放

        // 更新最终状态
        let mut status_guard = status.lock().unwrap();
        status_guard.is_running = false;

        match result {
            Ok(_) => {
                info!("优化完成");
            }
            Err(e) => {
                info!("优化失败: {}", e.message);
                status_guard.error = Some(e.message);
            }
        };
    });

    Json(ApiResponse::Success {
        result: "优化已开始，请通过轮询获取进度".to_string(),
    })
}

/// 获取优化状态（轮询端点）
pub async fn get_optimization_status(
    State(state): State<AppState>,
) -> Json<OptimizationStatus> {
    let status = state.optimization_status.lock().unwrap();
    Json(status.clone())
}

/// 主页面
pub async fn index() -> Html<&'static str> {
    Html(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>libchai API 服务器</title>
    <meta charset="utf-8">
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        code { background: #f4f4f4; padding: 2px 4px; border-radius: 3px; }
        pre { background: #f9f9f9; padding: 15px; border-radius: 5px; overflow-x: auto; }
        button { padding: 10px 15px; margin: 5px; cursor: pointer; }
        .status-panel { background: #f0f8ff; padding: 15px; border-radius: 5px; margin: 10px 0; }
        .progress { color: #007acc; }
        .error { color: #e74c3c; }
        .success { color: #27ae60; }
    </style>
</head>
<body>
    <h1>libchai API 服务器</h1>
    
    <h2>HTTP API 端点</h2>
    <ul>
        <li><code>POST /api/validate</code> - 验证配置</li>
        <li><code>POST /api/sync</code> - 同步参数</li>
        <li><code>POST /api/encode</code> - 编码评估</li>
        <li><code>POST /api/optimize</code> - 开始优化</li>
        <li><code>GET /api/status</code> - 获取优化状态（轮询）</li>
    </ul>
    
    <h2>静态文件服务</h2>
    <p><code>/*</code> - 提供 client 目录中的静态文件</p>
    
    <h2>测试工具</h2>
    <button onclick="testValidate()">测试验证</button>
    <button onclick="testSync()">测试同步</button>
    <button onclick="testEncode()">测试编码</button>
    <button onclick="testOptimize()">开始优化</button>
    <button onclick="startPolling()">开始轮询状态</button>
    <button onclick="stopPolling()">停止轮询</button>
    
    <div class="status-panel">
        <h3>优化状态：</h3>
        <div id="status">未知</div>
    </div>
    
    <h3>输出：</h3>
    <div id="output"></div>

    <script>
        let pollingInterval = null;
        
        function log(message) {
            const now = new Date().toLocaleTimeString();
            document.getElementById('output').innerHTML += `<p>[${now}] ${message}</p>`;
        }
        
        async function apiCall(endpoint, data) {
            try {
                const timeoutMs = 600000; // 10分钟，与服务器端一致
                
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
                
                const response = await fetch(`/api/${endpoint}`, {
                    method: data !== undefined ? 'POST' : 'GET',
                    headers: { 'Content-Type': 'application/json' },
                    body: data !== undefined ? JSON.stringify(data) : undefined,
                    signal: controller.signal
                });
                
                clearTimeout(timeoutId);
                const result = await response.json();
                
                if (result.type === 'success') {
                    log(`✅ ${endpoint}: ${JSON.stringify(result.result)}`);
                    return result.result;
                } else {
                    log(`❌ ${endpoint} 错误: ${result.error}`);
                    throw new Error(result.error);
                }
            } catch (error) {
                if (error.name === 'AbortError') {
                    log(`⏰ ${endpoint} 请求超时`);
                    throw new Error('请求超时，请稍后重试');
                } else {
                    log(`❌ ${endpoint} 网络错误: ${error.message}`);
                    throw error;
                }
            }
        }
        
        async function testValidate() {
            await apiCall('validate', {"version": "1.0"});
        }
        
        async function testSync() {
            await apiCall('sync', {
                配置: { version: "1.0" },
                词列表: [],
                原始键位分布信息: {},
                原始当量信息: {}
            });
        }
        
        async function testEncode() {
            log('🔄 开始编码评估（可能需要几分钟时间）...');
            try {
                await apiCall('encode', {});
            } catch (error) {
                // 错误已经在 apiCall 中处理了
            }
        }
        
        async function testOptimize() {
            const result = await apiCall('optimize', null);
            // 自动开始轮询状态
            if (!pollingInterval) {
                startPolling();
            }
        }
        
        async function pollStatus() {
            try {
                const status = await apiCall('status', undefined);
                updateStatusDisplay(status);
                
                // 如果优化完成或出错，停止轮询
                if (!status.is_running && (status.progress?.type === 'optimize_success' || status.error)) {
                    stopPolling();
                }
            } catch (error) {
                console.error('轮询状态失败:', error);
            }
        }
        
        function updateStatusDisplay(status) {
            const statusDiv = document.getElementById('status');
            
            if (status.is_running) {
                statusDiv.innerHTML = '<span class="progress">🔄 优化进行中...</span>';
                if (status.progress) {
                    statusDiv.innerHTML += `<br>进度: ${JSON.stringify(status.progress)}`;
                }
            } else if (status.progress?.type === 'optimize_success') {
                statusDiv.innerHTML = `<span class="success">✅ 优化完成</span><br>结果: ${JSON.stringify(status.progress)}`;
            } else if (status.error) {
                statusDiv.innerHTML = `<span class="error">❌ 优化失败</span><br>错误: ${status.error}`;
            } else {
                statusDiv.innerHTML = '⏸️ 空闲状态';
            }
        }
        
        function startPolling() {
            if (pollingInterval) {
                clearInterval(pollingInterval);
            }
            
            log('🔄 开始轮询优化状态...');
            pollingInterval = setInterval(pollStatus, 1000); // 每秒轮询一次
            pollStatus(); // 立即执行一次
        }
        
        function stopPolling() {
            if (pollingInterval) {
                clearInterval(pollingInterval);
                pollingInterval = null;
                log('⏹️ 停止轮询状态');
            }
        }
        
        // 页面加载时检查一次状态
        window.onload = () => {
            pollStatus();
        };
    </script>
</body>
</html>
    "#,
    )
}

/// 创建应用路由
pub fn create_app() -> Router {
    let state = AppState {
        api: Arc::new(Mutex::new(WebApi::new())),
        optimization_status: Arc::new(Mutex::new(OptimizationStatus {
            is_running: false,
            progress: None,
            error: None,
        })),
    };

    // 设置全局回调函数
    {
        let mut api = state.api.lock().unwrap();
        let status = state.optimization_status.clone();
        api.set_callback(move |消息| {
            // 只记录关键进度
            match 消息 {
                消息::Progress { steps, .. } => {
                    if steps % 100 == 0 {
                        // 每100步记录一次
                        info!("优化进度: {} 步", steps);
                    }
                }
                消息::BetterSolution { .. } => {
                    info!("发现更优解");
                }
                消息::Parameters { .. } => {
                    info!("设置优化参数");
                }
                _ => {} // 其他消息不记录，避免日志过多
            }

            // 更新状态
            let mut status_guard = status.lock().unwrap();
            let progress_msg = serde_json::json!(消息);
            status_guard.progress = Some(progress_msg);
        });
    }
    // 配置更详细的 CORS 设置
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .allow_private_network(true)
        .allow_credentials(false);

    Router::new()
        .route("/test", get(index))
        .route("/api/validate", post(validate_config))
        .route("/api/sync", post(sync_params))
        .route("/api/encode", post(encode_evaluate))
        .route("/api/optimize", post(start_optimize))
        .route("/api/status", get(get_optimization_status))
        .fallback_service(ServeDir::new("client"))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB 请求体限制
        .layer(cors)
        .layer(TimeoutLayer::new(Duration::from_secs(600))) // 10分钟超时，与编码任务一致
        .with_state(state)
}

/// 启动服务器
pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let app = create_app();
    let addr = format!("0.0.0.0:{port}");

    info!("Listening on: http://{addr}");
    info!("API Endpoints:");
    info!("   POST /api/validate    - 验证配置");
    info!("   POST /api/sync        - 同步参数");
    info!("   POST /api/encode      - 编码评估");
    info!("   POST /api/optimize    - 开始优化");
    info!("   GET  /api/status      - 获取优化状态");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    start_server(3200).await
}
