# MyHS - Python风格的HTTP文件服务器

🌐 一个使用Rust编写的轻量级HTTP文件服务器，提供类似Python `http.server`的功能体验。

## ✨ 功能特性

- 📁 **目录浏览** - 自动生成美观的目录索引页面
- 📥 **文件下载** - 支持直接下载文件，自动检测MIME类型
- 🔍 **路径导航** - 支持子目录浏览和上级目录返回
- 🛡️ **安全防护** - 内置路径遍历攻击防护
- 🚀 **高性能** - 基于Rust异步运行时，处理速度快
- 🎨 **现代界面** - 清爽的HTML界面设计
- 🌍 **跨域支持** - 内置CORS配置
- 📊 **结构化日志** - 详细的请求日志记录

## 🛠️ 技术栈

- **Rust** - 系统编程语言，提供高性能和内存安全
- **Axum** - 现代异步Web框架
- **Tokio** - 异步运行时
- **Tower-HTTP** - HTTP中间件和服务
- **Tracing** - 结构化日志记录
- **Serde** - 序列化和反序列化

## 📦 安装与编译

### 前置要求

- Rust 1.70+ (推荐使用最新稳定版)
- Cargo (Rust包管理器)

### 克隆项目

```bash
git clone <repository-url>
cd MyHS
```

### 编译项目

```bash
# 开发版本编译
cargo build

# 生产版本编译（优化）
cargo build --release
```

## 🚀 使用方法

### 开发模式运行

```bash
cargo run
```

### 生产模式运行

```bash
# 编译后运行可执行文件
./target/release/MyHS.exe
```

### 命令行参数

```bash
MyHS.exe [服务目录] [端口号]
```

**参数说明：**
- `服务目录` - 要服务的目录路径（可选，默认为当前目录）
- `端口号` - 服务器监听端口（可选，默认为8081）

**使用示例：**

```bash
# 在当前目录启动服务器，默认端口8081
MyHS.exe

# 服务指定目录
MyHS.exe C:\MyFiles

# 服务指定目录和端口
MyHS.exe C:\MyFiles 9000

# 仅指定端口（服务当前目录）
MyHS.exe . 9000
```

## 🌐 访问服务器

启动后，在浏览器中访问：

```
http://127.0.0.1:8081
```

或者使用自定义端口：

```
http://127.0.0.1:[您的端口号]
```

## 📋 功能说明

### 目录浏览
- 自动生成目录列表页面
- 显示文件和文件夹
- 支持文件大小显示
- 提供上级目录导航

### 文件下载
- 点击文件名直接下载
- 自动检测文件MIME类型
- 支持各种文件格式

### 安全特性
- 防止路径遍历攻击
- 只能访问指定目录及其子目录
- 安全的文件路径处理

## 🔧 开发说明

### 项目结构

```
MyHS/
├── src/
│   └── main.rs          # 主程序文件
├── static/              # 静态文件目录
│   ├── demo.html
│   ├── script.js
│   └── style.css
├── Cargo.toml           # 项目配置文件
├── Cargo.lock           # 依赖锁定文件
└── README.md            # 项目说明文档
```

### 主要依赖

```toml
[dependencies]
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1.0", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1.0", features = ["derive"] }
```

## 🆚 与Python http.server的对比

| 特性 | MyHS (Rust) | Python http.server |
|------|-------------|--------------------|
| 性能 | 🚀 高性能异步处理 | ⚡ 单线程同步处理 |
| 内存使用 | 💾 低内存占用 | 📈 相对较高 |
| 启动速度 | ⚡ 快速启动 | 🐌 较慢 |
| 安全性 | 🛡️ 内置安全防护 | ⚠️ 基础安全 |
| 界面 | 🎨 现代化设计 | 📄 简单列表 |
| 跨域支持 | ✅ 内置CORS | ❌ 需要额外配置 |
| 日志记录 | 📊 结构化日志 | 📝 简单日志 |

## 🤝 贡献指南

欢迎提交Issue和Pull Request！

1. Fork本项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启Pull Request

## 📄 许可证

本项目采用MIT许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Axum](https://github.com/tokio-rs/axum) - 优秀的异步Web框架
- [Tokio](https://tokio.rs/) - 强大的异步运行时
- [Tower](https://github.com/tower-rs/tower) - 模块化的网络服务库

---

**享受使用MyHS带来的高性能文件服务体验！** 🎉