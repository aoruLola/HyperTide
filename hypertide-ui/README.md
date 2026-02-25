# HyperTide UI

HyperTide 游戏资产版本控制系统的桌面客户端，基于 Tauri + React + TypeScript 构建。

## 技术栈

- **Tauri 2.0** - 轻量级桌面应用框架
- **React 18** - UI 框架
- **TypeScript** - 类型安全
- **TailwindCSS** - 样式框架
- **TanStack Query** - 数据获取和缓存
- **Zustand** - 状态管理
- **Axios** - HTTP 客户端
- **Lucide React** - 图标库

## 功能特性

### 1. 文件锁定管理
- 锁定文件防止冲突
- 查看所有锁定状态
- 解锁自己的文件
- 管理员强制解锁

### 2. 文件上传
- 拖拽上传文件
- 自动计算 BLAKE3 哈希
- 内容去重存储
- 上传进度显示

### 3. API 密钥管理
- 生成新的 API Key
- 查看所有密钥
- 撤销密钥
- 权限管理

## 开发环境要求

- Node.js 18+
- Rust 1.70+
- npm 或 yarn

## 快速开始

### 1. 安装依赖

```bash
npm install
```

### 2. 启动后端服务

确保 HyperTide 后端服务正在运行：

```bash
cd ../
cargo run
```

后端将在 `http://localhost:3000` 启动。

### 3. 启动开发服务器

```bash
npm run tauri dev
```

这将同时启动 Vite 开发服务器和 Tauri 应用。

### 4. 构建生产版本

```bash
npm run tauri build
```

构建产物将在 `src-tauri/target/release/bundle/` 目录下。

## 项目结构

```
hypertide-ui/
├── src/
│   ├── components/          # React 组件
│   │   ├── LockManager.tsx  # 文件锁定管理
│   │   ├── FileUploader.tsx # 文件上传
│   │   └── KeyManager.tsx   # 密钥管理
│   ├── lib/
│   │   ├── api.ts          # API 客户端
│   │   └── utils.ts        # 工具函数
│   ├── store/
│   │   └── useAppStore.ts  # 全局状态
│   ├── App.tsx             # 主应用组件
│   ├── main.tsx            # 入口文件
│   └── index.css           # 全局样式
├── src-tauri/              # Tauri 后端
│   ├── src/
│   │   └── lib.rs          # Rust 代码
│   ├── Cargo.toml          # Rust 依赖
│   └── tauri.conf.json     # Tauri 配置
└── package.json
```

## 配置

### API 地址

在 `.env` 文件中配置后端 API 地址：

```env
VITE_API_URL=http://localhost:3000
```

### 默认凭证

开发模式下使用以下默认凭证：

- API Key: `dev-master-key`
- User ID: `dev-user`

## 与 Electron 的对比

如果你之前使用过 Electron，以下是主要区别：

| 特性 | Tauri | Electron |
|------|-------|----------|
| 后端语言 | Rust | Node.js |
| 包体积 | ~3-5MB | ~50-100MB |
| 内存占用 | 更低 | 较高 |
| 启动速度 | 更快 | 较慢 |
| 安全性 | 更高 | 一般 |
| 生态系统 | 较新 | 成熟 |

## 常见问题

### Q: 如何调试 Tauri 应用？

A: 在开发模式下，按 `F12` 打开开发者工具。

### Q: 如何访问系统 API？

A: 在 `src-tauri/src/lib.rs` 中定义 Tauri 命令，然后在前端通过 `@tauri-apps/api` 调用。

### Q: 如何打包为不同平台？

A: 使用 `npm run tauri build -- --target <platform>`，例如：
- Windows: `--target x86_64-pc-windows-msvc`
- macOS: `--target x86_64-apple-darwin`
- Linux: `--target x86_64-unknown-linux-gnu`

## 下一步

- [ ] 添加文件浏览器
- [ ] 实现版本历史查看
- [ ] 添加实时同步状态
- [ ] 支持批量操作
- [ ] 添加搜索和过滤功能

## 许可证

MIT
