# HyperTide 部署教程：Ubuntu 服务端 + Windows 本地 CLI

这份教程面向一种最常见的内部部署方式：

- Ubuntu 机器负责运行 `hypertide` 服务端
- Windows 本地机器负责安装和使用 `ht` CLI

当前教程按“先跑通、再收紧”的方式写：

- 第 1 段先用开发模式把服务跑起来，便于联调和内部验证
- 最后单列生产化注意事项，告诉你上线前必须替换哪些默认值

## 1. 架构说明

这套部署现在已经拆成两条独立交付线：

- 服务端：`deploy/server/`
- CLI：`deploy/cli/`

服务端只需要部署：

- PostgreSQL
- JWT key bootstrap
- `hypertide` 容器

Windows 本地只需要拿到 `ht` 二进制，不需要 Docker，不需要服务端源码运行时。

## 2. Ubuntu 服务端部署

### 2.1 前置条件

Ubuntu 服务器需要：

- Docker Engine
- Docker Compose Plugin
- 能访问 GitHub 仓库或已经拿到仓库源码
- 对外开放服务端端口，默认是 `3000`

建议目录：

```bash
sudo mkdir -p /opt/hypertide
sudo chown "$USER":"$USER" /opt/hypertide
cd /opt/hypertide
```

### 2.2 获取代码

如果服务器可以直接拉仓库：

```bash
git clone https://github.com/aoruLola/HyperTide.git
cd HyperTide
git checkout codex/deploy-split-main
```

如果你后面把 PR 合进 `main`，这里就直接切到 `main`。

### 2.3 准备环境文件

先复制服务端专用环境模板：

```bash
cp deploy/server/.env.example deploy/server/.env
```

开发联调阶段，默认模板就能启动。关键默认值包括：

- `APP_PORT=3000`
- `POSTGRES_PORT=5432`
- `AUTH_PEPPER=hypertide-dev-pepper`
- development 默认 master key：`dev-master-key`

### 2.4 启动服务端

```bash
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env up -d --build
```

查看状态：

```bash
docker compose -f deploy/server/docker-compose.yml ps
```

你应该能看到：

- `postgres`
- `jwt-keys`
- `hypertide`

### 2.5 验证健康检查

在 Ubuntu 服务器本机验证：

```bash
curl http://127.0.0.1:3000/health/live
curl http://127.0.0.1:3000/health/ready
```

预期：

- `/health/live` 返回 `OK`
- `/health/ready` 返回成功状态

如果你要从局域网或公网访问，再额外确认服务器防火墙和云安全组已经放行 `3000/tcp`。

### 2.6 验证开发模式登录

当前默认开发模式下，服务端会接受内建 master key：

```text
dev-master-key
```

你可以先在 Ubuntu 本机验证认证接口：

```bash
curl -X POST http://127.0.0.1:3000/v2/auth/exchange-key \
  -H "Content-Type: application/json" \
  -d '{"api_key":"dev-master-key"}'
```

如果返回成功 JSON，说明服务端认证路径已经通了。

## 3. Windows 本地 CLI 安装

Windows 本地有两种方式，推荐第一种。

### 3.1 方式一：使用打包好的 CLI 压缩包

在一台能构建仓库的 Windows 机器上，运行：

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\cli\package.ps1
```

产物默认在：

```text
deploy\cli\dist\hypertide-cli-<version>-windows-x86_64.zip
```

把这个 zip 拷到你的 Windows 客户端，解压后拿到：

```text
ht.exe
```

建议放到例如：

```text
C:\Tools\HyperTide\
```

然后把这个目录加入 `PATH`。

验证：

```powershell
ht --version
ht --help
```

### 3.2 方式二：从源码直接运行

如果这台 Windows 本地本来就是开发机，也可以直接在仓库里运行：

```powershell
cargo run -p hypertide-cli --bin ht -- --help
```

这种方式适合开发联调，不推荐作为长期日常使用入口。

## 4. Windows 本地连接 Ubuntu 服务端

假设你的 Ubuntu 服务器地址是：

```text
http://192.168.1.50:3000
```

第一次登录：

```powershell
ht login --server http://192.168.1.50:3000 --token dev-master-key --repo demo-repo --branch main
```

如果只是开发联调，也可以走直连模式：

```powershell
ht login --server http://192.168.1.50:3000 --token dev-master-key --api-key-direct --repo demo-repo --branch main
```

然后走一条最小工作流：

```powershell
ht sync --repo demo-repo --branch main
ht checkout --repo demo-repo --branch main
ht status --repo demo-repo --branch main
```

如果你要验证一次完整提交链路，可以在一个测试目录里：

```powershell
New-Item -ItemType Directory -Force .\ht-demo | Out-Null
Set-Location .\ht-demo
"hello hypertide" | Set-Content .\hello.txt -NoNewline

ht login --server http://192.168.1.50:3000 --token dev-master-key --repo demo-repo --branch main
ht add --file .\hello.txt --asset-path Content/Demo/hello.txt
ht submit --repo demo-repo --branch main --message "add demo file"
ht sync --repo demo-repo --branch main
ht checkout --repo demo-repo --branch main
ht status --repo demo-repo --branch main
```

## 5. 常见问题

### 5.1 Windows 能访问 Ubuntu 但 `login` 失败

先确认这几项：

- Ubuntu 上 `/health/live` 和 `/health/ready` 都正常
- Windows 到 Ubuntu 的 `3000` 端口网络是通的
- 你没有把 `http://127.0.0.1:3000` 错写成 Ubuntu 本机回环地址

### 5.2 服务启动成功但健康检查不通过

优先看容器日志：

```bash
docker compose -f deploy/server/docker-compose.yml logs -f hypertide
docker compose -f deploy/server/docker-compose.yml logs -f postgres
```

通常优先排查：

- PostgreSQL 还没 ready
- JWT key 没生成
- 环境变量缺失或写错

### 5.3 Windows CLI 能登录但 `checkout` 或 `submit` 失败

先跑：

```powershell
ht status --repo demo-repo --branch main
ht diff --repo demo-repo --branch main
```

如果看到：

- `locked_by_other`：说明有锁冲突
- `stale_base`：说明本地基线过期，需要重新 `sync -> checkout`

## 6. 上线前必须替换的项

如果你要从“开发联调”切到“正式 Ubuntu 服务端”，不要继续用开发默认值。

至少要替换或设置这些项：

- `APP_ENV=production`
- `MASTER_KEY`
- `AUTH_PEPPER`
- `CORS_ALLOWED_ORIGINS`
- `HIGH_RISK_SIGNATURE_REQUIRED=true`
- `HIGH_RISK_SIGNING_SECRET`
- `WITNESS_KEYS`
- 自己生成并长期保存的 JWT key，而不是临时开发 key
- PostgreSQL 用户名和密码

当前代码里，`APP_ENV=production` 时如果仍然使用开发默认值，服务会直接拒绝启动。这是预期行为，不是 bug。

## 7. 推荐的最小部署顺序

如果你要一次性最稳地走完，推荐就是这 7 步：

1. 在 Ubuntu 拉代码并切到带部署拆分的分支
2. 复制 `deploy/server/.env.example` 为 `deploy/server/.env`
3. `docker compose ... up -d --build`
4. `curl /health/live` 和 `curl /health/ready`
5. 在 Windows 解压 `ht.exe`
6. 用 `ht login` 连接 Ubuntu 服务地址
7. 跑一次 `sync -> checkout -> status`
