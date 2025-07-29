use axum::{
    extract::{Path, Multipart},
    http::{StatusCode, HeaderMap, header},
    response::{Html, Response, IntoResponse},
    routing::{get, post},
    Router,
};
use std::{
    path::{Path as StdPath, PathBuf},
    env,
    io::Write,
};
use tokio::fs;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


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
        args[2].parse().unwrap_or(2333)
    } else {
        2333
    };

    // 验证目录是否存在
    if !serve_dir.exists() || !serve_dir.is_dir() {
        eprintln!("错误: 目录 '{}' 不存在或不是一个目录", serve_dir.display());
        std::process::exit(1);
    }

    println!("🌐 Python风格的HTTP文件服务器");
    println!("📁 服务目录: {}", serve_dir.display());
    println!("🚀 服务器地址: http://0.0.0.0:{}", port);
    println!("📋 功能:");
    println!("   • 目录浏览");
    println!("   • 文件下载");
    println!("   • 文件上传");
    println!("   • 自动索引页面");
    println!("   • 文件信息显示");
    println!("\n按 Ctrl+C 停止服务器\n");

    // 构建应用路由
    let app = Router::new()
        .route("/", get(serve_handler))
        .route("/*path", get(serve_handler))
        .route("/upload", post(upload_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(serve_dir);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
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
    _base_dir: &StdPath,
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

    // 添加文件上传表单
    let upload_form = format!(r#"
    <div class="upload-container">
        <h3>📤 文件上传</h3>
        <form id="uploadForm" action="/upload" method="post" enctype="multipart/form-data">
            <input type="hidden" name="current_path" value="{}">
            <div class="upload-box">
                <div class="file-input-container">
                    <input type="file" id="fileInput" name="file" class="file-input" multiple>
                    <label for="fileInput" class="file-label">选择文件</label>
                </div>
                <div class="file-actions">
                    <button type="button" id="clearButton" class="clear-button" style="display:none;">清除全部</button>
                    <button type="submit" class="upload-button">上传</button>
                </div>
            </div>
            <div id="fileList" class="file-list">
                <div class="no-files">未选中文件</div>
            </div>
        </form>
    </div>
    "#, current_path);

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
        .upload-container {{
            padding: 20px;
            background-color: #f8f9fa;
            border-top: 1px solid #dee2e6;
        }}
        .upload-container h3 {{
            margin-top: 0;
            color: #495057;
        }}
        .upload-box {{
            display: flex;
            align-items: center;
            flex-wrap: wrap;
            gap: 10px;
            padding: 15px;
            border: 2px dashed #ccc;
            border-radius: 8px;
            background-color: white;
        }}
        .file-input-container {{
            position: relative;
        }}
        .file-input {{
            position: absolute;
            width: 0.1px;
            height: 0.1px;
            opacity: 0;
            overflow: hidden;
            z-index: -1;
        }}
        .file-label {{
            display: inline-block;
            padding: 8px 16px;
            background-color: #007bff;
            color: white;
            border-radius: 4px;
            cursor: pointer;
            font-weight: 500;
            transition: background-color 0.2s;
        }}
        .file-label:hover {{
            background-color: #0069d9;
        }}
        .file-actions {{
            display: flex;
            gap: 10px;
        }}
        .file-list {{
            margin-top: 15px;
            max-height: 200px;
            overflow-y: auto;
            border: 1px solid #dee2e6;
            border-radius: 8px;
            background-color: #f8f9fa;
            box-shadow: inset 0 1px 3px rgba(0,0,0,0.1);
        }}
        .no-files {{
            padding: 15px;
            text-align: center;
            color: #6c757d;
            font-style: italic;
        }}
        .file-item {{
            display: grid;
            grid-template-columns: 1fr 80px 30px;
            align-items: center;
            padding: 10px 15px;
            border-bottom: 1px solid #dee2e6;
            transition: background-color 0.2s;
        }}
        .file-item:hover {{
            background-color: #e9ecef;
        }}
        .file-item:last-child {{
            border-bottom: none;
        }}
        .file-item-name {{
            font-weight: 500;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            padding-right: 10px;
        }}
        .file-item-size {{
            color: #6c757d;
            font-size: 0.9em;
            text-align: right;
            padding-right: 15px;
        }}
        .remove-file {{
            background-color: #f8f9fa;
            color: #dc3545;
            border: 1px solid #dc3545;
            border-radius: 50%;
            width: 24px;
            height: 24px;
            display: flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            font-size: 0.9em;
            font-weight: bold;
            transition: all 0.2s;
        }}
        .remove-file:hover {{
            background-color: #dc3545;
            color: white;
        }}
        .clear-button {{
            padding: 8px 16px;
            background-color: #dc3545;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-weight: 500;
            transition: background-color 0.2s;
        }}
        .clear-button:hover {{
            background-color: #c82333;
        }}
        .upload-button {{
            padding: 8px 16px;
            background-color: #28a745;
            color: white;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-weight: 500;
            transition: background-color 0.2s;
        }}
        .upload-button:hover {{
            background-color: #218838;
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
        {}
        <div class="footer">
            <p>⚡ Rust HTTP 文件服务器 - 类似 Python http.server</p>
        </div>
    </div>
    <script>
        document.addEventListener('DOMContentLoaded', function() {{
            const fileInput = document.getElementById('fileInput');
            const fileList = document.getElementById('fileList');
            const clearButton = document.getElementById('clearButton');
            const uploadForm = document.getElementById('uploadForm');
            
            // 格式化文件大小
            function formatFileSize(bytes) {{
                if (bytes === 0) return '0 B';
                const k = 1024;
                const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
                const i = Math.floor(Math.log(bytes) / Math.log(k));
                return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
            }}
            
            // 更新文件列表
            function updateFileList() {{
                fileList.innerHTML = '';
                
                if (fileInput.files.length === 0) {{
                    clearButton.style.display = 'none';
                    const noFiles = document.createElement('div');
                    noFiles.className = 'no-files';
                    noFiles.textContent = '未选中文件';
                    fileList.appendChild(noFiles);
                    return;
                }}
                
                clearButton.style.display = 'inline-block';
                
                // 创建一个文档片段来提高性能
                const fragment = document.createDocumentFragment();
                
                for (let i = 0; i < fileInput.files.length; i++) {{
                    const file = fileInput.files[i];
                    const fileItem = document.createElement('div');
                    fileItem.className = 'file-item';
                    fileItem.dataset.index = i;
                    
                    const fileName = document.createElement('div');
                    fileName.className = 'file-item-name';
                    fileName.textContent = file.name;
                    
                    const fileSize = document.createElement('div');
                    fileSize.className = 'file-item-size';
                    fileSize.textContent = formatFileSize(file.size);
                    
                    const removeButton = document.createElement('button');
                    removeButton.className = 'remove-file';
                    removeButton.textContent = '×';
                    removeButton.type = 'button';
                    removeButton.title = '移除文件';
                    removeButton.addEventListener('click', function() {{
                        removeFile(i);
                    }});
                    
                    fileItem.appendChild(fileName);
                    fileItem.appendChild(fileSize);
                    fileItem.appendChild(removeButton);
                    fragment.appendChild(fileItem);
                }}
                
                fileList.appendChild(fragment);
            }}
            
            // 移除单个文件
            function removeFile(index) {{
                const dt = new DataTransfer();
                const files = fileInput.files;
                
                for (let i = 0; i < files.length; i++) {{
                    if (i !== index) {{
                        dt.items.add(files[i]);
                    }}
                }}
                
                fileInput.files = dt.files;
                updateFileList();
            }}
            
            // 监听文件选择变化
            fileInput.addEventListener('change', function() {{
                updateFileList();
            }});
            
            // 清除所有文件
            clearButton.addEventListener('click', function() {{
                fileInput.value = '';
                updateFileList();
            }});
        }});
    </script>
</body>
</html>
    "#, title, current_path, parent_link, file_rows, upload_form))
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

// 处理文件上传
#[axum::debug_handler]
async fn upload_handler(
    axum::extract::State(base_dir): axum::extract::State<PathBuf>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut current_path = String::new();
    let mut success_count = 0;
    let mut total_files = 0;

    // 首先获取当前路径
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "current_path" {
            if let Ok(data) = field.text().await {
                current_path = data;
                break;
            }
        }
    }

    // 确定目标目录
    let target_dir = if current_path.is_empty() {
        base_dir.clone()
    } else {
        base_dir.join(&current_path)
    };

    // 安全检查：确保目标目录在基础目录内
    if !target_dir.starts_with(&base_dir) {
        return (StatusCode::FORBIDDEN, "访问被拒绝").into_response();
    }

    // 处理所有文件
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        
        if name == "file" {
            if let Some(file_name) = field.file_name() {
                total_files += 1;
                let file_name = file_name.to_string();
                
                if let Ok(data) = field.bytes().await {
                    let file_path = target_dir.join(&file_name);
                    
                    // 写入文件
                    match std::fs::File::create(&file_path) {
                        Ok(mut file) => {
                            if file.write_all(&data).is_ok() {
                                success_count += 1;
                            }
                        },
                        Err(_) => {}
                    }
                }
            }
        }
    }

    // 上传后重定向回原目录
    let redirect_path = if current_path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", current_path)
    };

    if success_count > 0 {
        let message = if success_count == total_files {
            if total_files == 1 {
                "文件上传成功".to_string()
            } else {
                format!("所有{}个文件上传成功", total_files)
            }
        } else {
            format!("{}个文件中的{}个上传成功", total_files, success_count)
        };
        
        (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, redirect_path)],
            message
        ).into_response()
    } else {
        (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, redirect_path)],
            "文件上传失败"
        ).into_response()
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
