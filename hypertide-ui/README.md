# HyperTide UI

HyperTide 游戏资产版本控制系统的桌面客户端，基于 Tauri + React + TypeScript + Hero UI 构建。

## 🎨 UI 设计理念

采用类似 Perforce/SVN/Git GUI 的专业版本控制界面：
- 左侧导航栏 - 快速切换功能模块
- 主内容区 - 工作区、表格视图、详情面板
- 顶部状态栏 - 连接状态、用户信息
- 底部状态栏 - 实时统计信息

## 技术栈

- **Tauri 2.0** - 轻量级桌面应用框架
- **React 18** - UI 框架
- **TypeScript** - 类型安全
- **Hero UI** - 现代化 React 组件库（原 NextUI）
- **React Router** - 路由管理
- **TailwindCSS** - 样式框架
- **TanStack Query** - 数据获取和缓存
- **Zustand** - 状态管理
- **Axios** - HTTP 客户端
- **Lucide React** - 图标库
- **Framer Motion** - 动画库

## 功能特性

### 1. 工作区 (`/`)
- 文件树浏览
- 文件详情查看
- 锁定状态显示
- 快速操作（锁定、下载、上传）

### 2. 文件锁定管理 (`/locks`)
- 查看所有锁定文件
- 锁定新文件
- 解锁自己的文件
- 管理员强制解锁
- 实时搜索和过滤

### 3. 文件上传 (`/upload`)
- 批量文件选择
- 上传进度显示
- 自动计算 BLAKE3 哈希
- 内容去重存储
- 上传结果展示

### 4. 文件下载 (`/download`)
- 通过哈希下载文件
- 下载历史记录

### 5. 文件搜索 (`/search`)
- 按路径搜索
- 按哈希搜索
- 高级过滤

### 6. 操作历史 (`/history`)
- 查看所有操作记录
- 时间线视图

### 7. API 密钥管理 (`/keys`)
- 生成新的 API Key
- 查看所有密钥
- 撤销密钥
- 权限管理
- 一键复制

## 开发环境要求

- Node.js 18+
- Rust 1.70+
- npm 或 yarn

## 快速开始

### 1. 安装依赖

```bash
cd hypertide-ui
npm install
```

### 2. 启动后端服务

确保 HyperTide 后端服务正在运行：

```bash
cd ../
cargo run -p hypertide-server --bin hypertide
```

后端将在 `http://localhost:3000` 启动。

### 后端命名约定

- Rust workspace packages: `hypertide-server` and `hypertide-cli`
- Server binary: `hypertide`
- CLI binary: `ht`

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
│   │   ├── Sidebar.tsx      # 左侧导航栏
│   │   ├── Topbar.tsx       # 顶部状态栏
│   │   └── StatusBar.tsx    # 底部状态栏
│   ├── layouts/
│   │   └── MainLayout.tsx   # 主布局
│   ├── pages/               # 页面组件
│   │   ├── Workspace.tsx    # 工作区
│   │   ├── LocksPage.tsx    # 锁定管理
│   │   ├── UploadPage.tsx   # 文件上传
│   │   ├── DownloadPage.tsx # 文件下载
│   │   ├── SearchPage.tsx   # 文件搜索
│   │   ├── HistoryPage.tsx  # 操作历史
│   │   └── KeysPage.tsx     # 密钥管理
│   ├── lib/
│   │   ├── api.ts          # API 客户端
│   │   └── utils.ts        # 工具函数
│   ├── store/
│   │   └── useAppStore.ts  # 全局状态
│   ├── router.tsx          # 路由配置
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

## Hero UI 组件使用

Hero UI 提供了丰富的现代化组件：

- **Table** - 数据表格（锁定列表、密钥列表）
- **Card** - 卡片容器
- **Button** - 按钮（支持加载状态、图标）
- **Input** - 输入框（支持前缀图标）
- **Chip** - 标签（状态显示）
- **Progress** - 进度条（上传进度）
- **Spinner** - 加载动画
- **Tooltip** - 提示框

所有组件都支持深色模式，并与 Tailwind CSS 完美集成。

## 与 Electron 的对比

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

### Q: Hero UI 组件如何自定义主题？

A: 在 `tailwind.config.js` 中配置 Hero UI 插件的 themes 选项。

## 路由说明

应用使用 React Router 进行页面导航：

- `/` - 工作区（文件树浏览）
- `/locks` - 锁定管理
- `/upload` - 文件上传
- `/download` - 文件下载
- `/search` - 文件搜索
- `/history` - 操作历史
- `/keys` - API 密钥管理

## 下一步

- [x] 基础路由和导航
- [x] Hero UI 组件集成
- [x] 锁定管理（表格视图）
- [x] 文件上传（批量支持）
- [x] 密钥管理（完整 CRUD）
- [ ] 文件浏览器（完整实现）
- [ ] 实时同步状态
- [ ] 版本历史查看
- [ ] 搜索和过滤功能
- [ ] 操作历史记录

## 许可证

MIT
