use crate::config::{目标配置, 配置};
use crate::interfaces::server::WebApi;
use axum::extract::DefaultBodyLimit;
use axum::http::Method;
use axum::http::StatusCode;
use axum::{
    extract::State,
    response::{Html, sse::{Event, KeepAlive, Sse}},
    routing::{get, post},
    Json, Router,
};
use crate::interfaces::{默认输入};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
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
    /// 全局 WebApi 实例（使用 RwLock 支持异步并发）
    pub api: Arc<RwLock<WebApi>>,
    /// 优化状态（使用 RwLock 支持异步并发）
    pub optimization_status: Arc<RwLock<OptimizationStatus>>,
    /// WebSocket 广播发送器
    pub status_broadcast: broadcast::Sender<OptimizationStatus>,
    /// MPSC 发送器（用于从同步回调发送）
    pub status_mpsc: mpsc::UnboundedSender<OptimizationStatus>,
}

/// 优化状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OptimizationStatus {
    /// 空闲状态
    Idle,
    /// 运行中
    Running {
        message: serde_json::Value,
    },
    /// 已完成
    Completed {
        final_message: Option<serde_json::Value>,
    },
    /// 失败
    Failed {
        error: String,
    },
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
            // 使用写锁，异步等待
            let mut api = state.api.write().await;
            let result = api.sync(图形界面参数);
            drop(api); // 显式释放锁

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
    match serde_json::from_value::<目标配置>(objective) {
        Ok(目标函数配置) => {
            // 使用读锁，允许多个并发读取
            let api = state.api.read().await;
            let result = api.encode_evaluate(目标函数配置);
            drop(api); // 显式释放锁

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
        let status = state.optimization_status.read().await;
        if matches!(*status, OptimizationStatus::Running { .. }) {
            return Json(ApiResponse::Error {
                error: "优化已在进行中".to_string(),
            });
        }
    }

    // 设置运行状态
    {
        let mut status = state.optimization_status.write().await;
        *status = OptimizationStatus::Running {
            message: serde_json::json!({"info": "优化已启动"}),
        };
        match state.status_broadcast.send(status.clone()) {
            Ok(count) => info!("[OPTIMIZE] 初始状态广播成功，{} 个接收者", count),
            Err(_) => info!("[OPTIMIZE] 初始状态广播失败：没有接收者"),
        }
    }

    let api = state.api.clone();
    let status = state.optimization_status.clone();
    let broadcast = state.status_broadcast.clone();

    // 在后台启动优化任务
    tokio::spawn(async move {
        // 使用 spawn_blocking 运行同步阻塞的优化任务
        let api_clone = api.clone();
        let result = tokio::task::spawn_blocking(move || {
            // 在阻塞任务中使用 blocking 方式获取锁
            let api_guard = api_clone.blocking_read();
            api_guard.optimize()
        }).await;

        // 处理结果
        let final_status = match result {
            Ok(Ok(_)) => {
                info!("优化完成");
                OptimizationStatus::Completed {
                    final_message: None,
                }
            }
            Ok(Err(e)) => {
                info!("优化失败: {}", e.message);
                OptimizationStatus::Failed {
                    error: e.message,
                }
            }
            Err(e) => {
                info!("优化任务崩溃: {:?}", e);
                OptimizationStatus::Failed {
                    error: format!("任务崩溃: {:?}", e),
                }
            }
        };

        {
            let mut status_guard = status.write().await;
            *status_guard = final_status.clone();
        }

        // 广播最终状态
        match broadcast.send(final_status) {
            Ok(count) => info!("[OPTIMIZE] 最终状态广播成功，{} 个接收者", count),
            Err(_) => info!("[OPTIMIZE] 最终状态广播失败：没有接收者"),
        }
    });

    Json(ApiResponse::Success {
        result: "优化已启动".to_string(),
    })
}

/// SSE 处理函数
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    // 获取当前状态
    let initial_status = {
        let status = state.optimization_status.read().await;
        status.clone()
    };
    
    // 订阅广播通道
    let mut broadcast_rx = state.status_broadcast.subscribe();
    
    let stream = async_stream::stream! {
        // 发送初始状态
        if let Ok(json) = serde_json::to_string(&initial_status) {
            info!("[SSE] 连接建立，发送初始状态");
            yield Ok(Event::default().data(json));
        }
        
        // 持续接收广播消息
        let mut msg_count = 0;
        loop {
            match broadcast_rx.recv().await {
                Ok(status) => {
                    msg_count += 1;
                    if let Ok(json) = serde_json::to_string(&status) {
                        yield Ok(Event::default().data(json));
                    }
                }
                Err(_) => {
                    info!("[SSE] 连接关闭，共发送 {} 条消息", msg_count);
                    break;
                }
            }
        }
    };
    
    Sse::new(stream).keep_alive(KeepAlive::default())
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
        <li><code>GET /sse/status</code> - SSE 实时状态推送</li>
    </ul>
    
    <h2>静态文件服务</h2>
    <p><code>/*</code> - 提供 client 目录中的静态文件</p>
    
    <h2>测试工具</h2>
    <button onclick="testValidate()">测试验证</button>
    <button onclick="testSync()">测试同步</button>
    <button onclick="testEncode()">测试编码</button>
    <button onclick="testOptimize()">开始优化</button>
    <button onclick="reconnectWebSocket()">重新连接 SSE</button>
    
    <div class="status-panel">
        <h3>优化状态：</h3>
        <div id="status">未知</div>
    </div>
    
    <h3>输出：</h3>
    <div id="output"></div>

    <script>
        let eventSource = null;
        
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
        
        function connectSSE() {
            // 关闭现有连接
            if (eventSource) {
                eventSource.close();
                eventSource = null;
            }
            
            const sseUrl = '/sse/status';
            log(`🔌 连接 SSE: ${sseUrl}`);
            
            eventSource = new EventSource(sseUrl);
            
            eventSource.onopen = () => {
                log('✅ SSE 已连接');
            };
            
            eventSource.onmessage = (event) => {
                try {
                    log(`📨 收到 SSE 消息: ${event.data.substring(0, 100)}...`);
                    const status = JSON.parse(event.data);
                    updateStatusDisplay(status);
                } catch (error) {
                    console.error('解析 SSE 消息失败:', error);
                    log(`❌ 消息解析失败: ${error.message}`);
                }
            };
            
            eventSource.onerror = (error) => {
                log('❌ SSE 错误，将自动重连...');
                console.error('SSE error:', error);
                // EventSource 会自动重连，不需要手动处理
            };
        }
        
        function reconnectWebSocket() {
            log('🔄 手动重新连接 SSE...');
            connectSSE();
        }
        
        function updateStatusDisplay(status) {
            const statusDiv = document.getElementById('status');
            
            switch (status.status) {
                case 'idle':
                    statusDiv.innerHTML = '⏸️ 空闲状态';
                    break;
                    
                case 'running':
                    statusDiv.innerHTML = '<span class="progress">🔄 优化进行中...</span>';
                    if (status.message) {
                        const msg = status.message;
                        let details = '';
                        
                        if (msg.type === 'progress') {
                            details = `<br>步数: ${msg.steps}, 温度: ${msg.temperature.toFixed(4)}, 指标: ${msg.metric}`;
                        } else if (msg.type === 'better_solution') {
                            details = `<br>✨ 发现更优解！指标: ${msg.metric}`;
                        } else if (msg.type === 'parameters') {
                            details = `<br>参数: T_max=${msg.t_max.toFixed(2)}, T_min=${msg.t_min.toFixed(6)}`;
                        } else {
                            details = `<br>${JSON.stringify(msg)}`;
                        }
                        
                        statusDiv.innerHTML += details;
                    }
                    break;
                    
                case 'completed':
                    statusDiv.innerHTML = '<span class="success">✅ 优化完成</span>';
                    if (status.final_message) {
                        statusDiv.innerHTML += `<br>结果: ${JSON.stringify(status.final_message)}`;
                    }
                    break;
                    
                case 'failed':
                    statusDiv.innerHTML = `<span class="error">❌ 优化失败</span><br>错误: ${status.error}`;
                    break;
                    
                default:
                    statusDiv.innerHTML = `未知状态: ${JSON.stringify(status)}`;
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
            await apiCall('optimize', null);
        }
        
        // 页面加载时连接 SSE
        window.onload = () => {
            connectSSE();
        };
        
        // 页面卸载时关闭 SSE
        window.onbeforeunload = () => {
            if (eventSource) {
                eventSource.close();
            }
        };
    </script>
</body>
</html>
    "#,
    )
}

/// 创建应用路由
pub fn create_app() -> Router {
    // 创建广播通道（容量设置为 100）
    let (tx, _rx) = broadcast::channel(100);
    
    // 创建 MPSC 通道用于从同步回调发送
    let (mpsc_tx, mut mpsc_rx) = mpsc::unbounded_channel::<OptimizationStatus>();
    
    let state = AppState {
        api: Arc::new(RwLock::new(WebApi::new())),
        optimization_status: Arc::new(RwLock::new(OptimizationStatus::Idle)),
        status_broadcast: tx.clone(),
        status_mpsc: mpsc_tx.clone(),
    };
    
    // 启动转发任务：从 MPSC 转发到 broadcast
    let broadcast_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(status) = mpsc_rx.recv().await {
            let _ = broadcast_clone.send(status);
        }
    });

    // 设置全局回调函数
    {
        // 使用 try_write 因为在异步上下文中不能使用 blocking_write
        // 这里是初始化阶段，不会有其他线程竞争，所以 unwrap 是安全的
        let mut api = state.api.try_write().expect("初始化时获取 API 写锁失败");
        let status = state.optimization_status.clone();
        let mpsc_sender = mpsc_tx.clone();
        
        api.set_callback(move |消息| {
            // 将消息转换为 JSON
            let progress_msg = serde_json::json!(消息);
            
            // 更新状态
            let new_status = OptimizationStatus::Running {
                message: progress_msg,
            };
            
            // 只在重要进度时记录
            match 消息 {
                crate::interfaces::消息::Progress { steps, .. } => {
                    if steps % 100 == 0 {
                        info!("[CALLBACK] 优化进度: {} 步", steps);
                    }
                }
                crate::interfaces::消息::BetterSolution { .. } => {
                    info!("[CALLBACK] 发现更优解");
                }
                crate::interfaces::消息::Parameters { .. } => {
                    info!("[CALLBACK] 设置优化参数");
                }
                _ => {}
            }
            
            // 更新共享状态（使用 blocking_write 因为回调在同步上下文中）
            {
                let mut status_guard = status.blocking_write();
                *status_guard = new_status.clone();
            }
            
            // 通过 MPSC 发送（可以从任何线程调用）
            let _ = mpsc_sender.send(new_status);
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
        .route("/sse/status", get(sse_handler))
        .fallback_service(ServeDir::new("client"))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB 请求体限制
        .layer(cors)
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(600))) // 10分钟超时，与编码任务一致
        .with_state(state)
}

/// 尝试绑定可用端口
async fn bind_available_port(
    preferred_port: u16,
) -> Result<(tokio::net::TcpListener, u16), Box<dyn std::error::Error>> {
    // 首先尝试首选端口
    let addr = format!("0.0.0.0:{}", preferred_port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!("成功绑定到首选端口: {}", preferred_port);
            return Ok((listener, preferred_port));
        }
        Err(e) => {
            info!("端口 {} 已被占用: {}", preferred_port, e);
        }
    }

    // 如果首选端口被占用，尝试附近的端口
    for offset in 1..=50 {
        let port = preferred_port + offset;
        if port < preferred_port {
            break;
        }

        let addr = format!("0.0.0.0:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                info!("成功绑定到替代端口: {}", port);
                return Ok((listener, port));
            }
            Err(_) => {
                // 继续尝试下一个端口
            }
        }
    }

    // 如果向上寻找失败，尝试向下寻找
    for offset in 1..=50 {
        if preferred_port < offset {
            break;
        }

        let port = preferred_port - offset;
        if port < 1024 {
            // 避免使用系统保留端口
            break;
        }

        let addr = format!("0.0.0.0:{}", port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => {
                info!("成功绑定到替代端口: {}", port);
                return Ok((listener, port));
            }
            Err(_) => {
                // 继续尝试下一个端口
            }
        }
    }

    // 最后尝试让系统自动分配端口
    match tokio::net::TcpListener::bind("0.0.0.0:0").await {
        Ok(listener) => {
            let actual_port = listener.local_addr()?.port();
            info!("使用系统自动分配的端口: {}", actual_port);
            Ok((listener, actual_port))
        }
        Err(e) => Err(format!("无法绑定到任何端口: {}", e).into()),
    }
}

/// 启动服务器
pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let app = create_app();

    // 尝试绑定端口，如果失败则尝试其他端口
    let (listener, actual_port) = bind_available_port(port).await?;

    info!("Listening on: http://127.0.0.1:{}", actual_port);
    info!("API Endpoints:");
    info!("   POST /api/validate    - 验证配置");
    info!("   POST /api/sync        - 同步参数");
    info!("   POST /api/encode      - 编码评估");
    info!("   POST /api/optimize    - 开始优化");
    info!("   GET  /sse/status      - SSE 实时状态推送");

    axum::serve(listener, app).await?;

    Ok(())
}
