use axum::{
    extract::{Path, Query},
    http::{StatusCode, Uri, HeaderMap, header},
    response::{Html, Response, IntoResponse},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path as StdPath, PathBuf},
    env,
};
use tokio::fs;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Deserialize)]
struct QueryParams {
    path: Option<String>,
}

struct FileInfo {
    name: String,
    is_dir: bool,
    size: Option<u64>,
    modified: Option<String>,
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "myhs=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 获取命令行参数或使用当前目录
    let args: Vec<String> = env::args().collect();
    let serve_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir().unwrap()
    };

    let port = if args.len() > 2 {
        args[2].parse().unwrap_or(8081)
    } else {
        8081
    };

    // 验证目录是否存在
    if !serve_dir.exists() || !serve_dir.is_dir() {
        eprintln!("错误: 目录 '{}' 不存在或不是一个目录", serve_dir.display());
        std::process::exit(1);
    }

    println!("🌐 Python风格的HTTP文件服务器");
    println!("📁 服务目录: {}", serve_dir.display());
    println!("🚀 服务器地址: http://127.0.0.1:{}", port);
    println!("📋 功能:");
    println!("   • 目录浏览");
    println!("   • 文件下载");
    println!("   • 自动索引页面");
    println!("   • 文件信息显示");
    println!("\n按 Ctrl+C 停止服务器\n");

    // 构建应用路由
    let app = Router::new()
        .route("/", get(serve_handler))
        .route("/*path", get(serve_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(serve_dir);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

// 主要的文件服务处理器
async fn serve_handler(
    path: Option<Path<String>>,
    axum::extract::State(base_dir): axum::extract::State<PathBuf>,
) -> impl IntoResponse {
    let path_str = path.map(|Path(p)| p).unwrap_or_default();
    let requested_path = if path_str.is_empty() {
        base_dir.clone()
    } else {
        base_dir.join(&path_str)
    };

    // 安全检查：防止路径遍历攻击
    if !requested_path.starts_with(&base_dir) {
        return (StatusCode::FORBIDDEN, "访问被拒绝").into_response();
    }

    if !requested_path.exists() {
        return (StatusCode::NOT_FOUND, "文件或目录不存在").into_response();
    }

    if requested_path.is_dir() {
        // 如果是目录，生成目录列表页面
        match generate_directory_listing(&requested_path, &base_dir, &path_str).await {
            Ok(html) => Html(html).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "无法读取目录").into_response(),
        }
    } else {
        // 如果是文件，提供文件下载
        match serve_file(&requested_path).await {
            Ok(response) => response,
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "无法读取文件").into_response(),
        }
    }
}

// 生成目录列表页面
async fn generate_directory_listing(
    dir_path: &StdPath,
    base_dir: &StdPath,
    current_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut entries = fs::read_dir(dir_path).await?;
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().to_string();
        
        let file_info = FileInfo {
            name: name.clone(),
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() { Some(metadata.len()) } else { None },
            modified: metadata.modified().ok().map(|time| {
                format!("{:?}", time)
            }),
        };

        if metadata.is_dir() {
            dirs.push(file_info);
        } else {
            files.push(file_info);
        }
    }

    // 排序：目录在前，文件在后，都按名称排序
    dirs.sort_by(|a, b| a.name.cmp(&b.name));
    files.sort_by(|a, b| a.name.cmp(&b.name));

    let title = if current_path.is_empty() {
        "目录索引 /".to_string()
    } else {
        format!("目录索引 /{}", current_path)
    };

    let parent_link = if current_path.is_empty() {
        String::new()
    } else {
        let parent_path = StdPath::new(current_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        format!(
            "<tr><td><a href='/{}'><strong>📁 ../</strong></a></td><td>-</td><td>目录</td></tr>",
            parent_path
        )
    };

    let mut file_rows = String::new();
    
    // 添加目录
    for dir in dirs {
        let link_path = if current_path.is_empty() {
            dir.name.clone()
        } else {
            format!("{}/{}", current_path, dir.name)
        };
        file_rows.push_str(&format!(
            "<tr><td><a href='/{}'><strong>📁 {}/</strong></a></td><td>-</td><td>目录</td></tr>",
            link_path, dir.name
        ));
    }

    // 添加文件
    for file in files {
        let link_path = if current_path.is_empty() {
            file.name.clone()
        } else {
            format!("{}/{}", current_path, file.name)
        };
        let size_str = file.size.map_or("-".to_string(), |s| format_file_size(s));
        file_rows.push_str(&format!(
            "<tr><td><a href='/{}'><strong>📄 {}</strong></a></td><td>{}</td><td>文件</td></tr>",
            link_path, file.name, size_str
        ));
    }

    Ok(format!(r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .container {{
            max-width: 1000px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            overflow: hidden;
        }}
        .header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px;
            text-align: center;
        }}
        .header h1 {{
            margin: 0;
            font-size: 1.8rem;
        }}
        .path {{
            background: #f8f9fa;
            padding: 15px 20px;
            border-bottom: 1px solid #dee2e6;
            font-family: monospace;
            color: #495057;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
        }}
        th, td {{
            text-align: left;
            padding: 12px 20px;
            border-bottom: 1px solid #dee2e6;
        }}
        th {{
            background-color: #f8f9fa;
            font-weight: 600;
            color: #495057;
        }}
        tr:hover {{
            background-color: #f8f9fa;
        }}
        a {{
            color: #007bff;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}
        .footer {{
            padding: 20px;
            text-align: center;
            color: #6c757d;
            font-size: 0.9rem;
            background: #f8f9fa;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🌐 HTTP 文件服务器</h1>
        </div>
        <div class="path">
            <strong>当前路径:</strong> /{}
        </div>
        <table>
            <thead>
                <tr>
                    <th>名称</th>
                    <th>大小</th>
                    <th>类型</th>
                </tr>
            </thead>
            <tbody>
                {}
                {}
            </tbody>
        </table>
        <div class="footer">
            <p>⚡ Rust HTTP 文件服务器 - 类似 Python http.server</p>
        </div>
    </div>
</body>
</html>
    "#, title, current_path, parent_link, file_rows))
}

// 提供文件下载服务
async fn serve_file(file_path: &StdPath) -> Result<Response, Box<dyn std::error::Error>> {
    let contents = fs::read(file_path).await?;
    let content_type = guess_content_type(file_path);
    
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    
    // 添加文件名到Content-Disposition头
    if let Some(filename) = file_path.file_name() {
        let disposition = format!(
            "inline; filename=\"{}\"", 
            filename.to_string_lossy()
        );
        headers.insert(header::CONTENT_DISPOSITION, disposition.parse().unwrap());
    }
    
    Ok((headers, contents).into_response())
}

// 格式化文件大小
fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// 根据文件扩展名猜测MIME类型
fn guess_content_type(file_path: &StdPath) -> &'static str {
    match file_path.extension().and_then(|ext| ext.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        Some("md") => "text/markdown; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("tar") => "application/x-tar",
        Some("gz") => "application/gzip",
        Some("mp4") => "video/mp4",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        _ => "application/octet-stream",
    }
}
