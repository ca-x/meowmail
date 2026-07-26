# 妙邮 / Meowmail

妙邮（Meowmail）是一个自托管、多用户、多邮件账户的 Web 邮件客户端。后端使用 Rust、Axum、SeaORM 与 SQLite，前端使用 React 19 与 Vite。生产构建会把完整 Web 资源嵌入 Rust 可执行文件，运行时无需 Node.js 或独立静态文件服务器。

`0.2.0` 是首个正式版本，不包含 `0.1.x` 数据库迁移逻辑。

## 界面预览

### PC 端

登录：

![妙邮 PC 登录界面](docs/assets/screenshots/login.png)

三栏工作区：

![妙邮 PC 三栏工作区](docs/assets/screenshots/workspace.png)

个人资料、主题与安全设置：

![妙邮 PC 设置界面](docs/assets/screenshots/settings.png)

### 移动端

<table>
  <tr>
    <th>登录</th>
    <th>收件箱</th>
    <th>个人设置</th>
  </tr>
  <tr>
    <td><img src="docs/assets/screenshots/mobile-login.png" width="240" alt="妙邮移动端登录界面"></td>
    <td><img src="docs/assets/screenshots/mobile-inbox.png" width="240" alt="妙邮移动端收件箱界面"></td>
    <td><img src="docs/assets/screenshots/mobile-settings.png" width="240" alt="妙邮移动端设置界面"></td>
  </tr>
</table>

## 核心能力

- 多用户：管理员与普通用户的邮件账户、邮件、通知设置和清理规则相互隔离
- 多邮件账户：独立 IMAP / SMTP 配置、默认账户与账户切换
- 每个邮件账户可独立使用直连、HTTP CONNECT 或 SOCKS5 代理，并支持代理认证
- 本地账号、OIDC 或混合登录；无管理员时可让首位 OIDC 用户自动成为管理员
- 可选个人 PIN 应用锁；PIN 只用于登录后的锁定/解锁，不是主登录凭据
- 用户头像、昵称及个人设置
- 加密配置导入/导出，可选择资料、邮件账户、通知、邮件保留与清理规则
- 管理员可选择“仅我的配置”或“所有用户”；普通用户只能操作自己的配置
- 服务器删除邮件后可保留本地副本；支持按账户、发件人、主题、正文与邮件年龄自动清理
- 新邮件自定义命令通知和 HTTP POST Webhook，支持 `{account}`、`{sender}`、`{subject}` 等模板
- 中文 / English，以及跟随系统、浅色、深色主题
- 桌面三栏、平板双栏和移动端列表/详情布局
- 单文件二进制、Docker amd64/arm64 镜像与自动化发布

当前邮件服务器认证使用邮箱密码或服务商提供的应用专用密码，尚未实现邮件服务商 OAuth2。同步范围目前是每个账户 INBOX 中最近 50 封邮件。

## 快速启动

要求：Rust 1.94、Node.js 26.4、npm 12.0.1。

### 本地账号模式

```bash
export MEOWMAIL_AUTH_MODE=local
export MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME=admin
export MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD='请换成足够长的随机密码'
npm --prefix web ci --ignore-scripts
make dev
```

浏览器打开 `http://127.0.0.1:5173/login`。Vite 会把 `/api` 请求代理到监听 `0.0.0.0:8080` 的 Rust 后端。

生产单文件构建：

```bash
make build
MEOWMAIL_AUTH_MODE=local \
MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME=admin \
MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD='请换成足够长的随机密码' \
./target/release/meowmail
```

打开 `http://127.0.0.1:8080/login`。

### OIDC 模式

```bash
export MEOWMAIL_AUTH_MODE=oidc
export MEOWMAIL_OIDC_ISSUER='https://id.example.com'
export MEOWMAIL_OIDC_CLIENT_ID='meowmail'
export MEOWMAIL_OIDC_CLIENT_SECRET='provider-client-secret'
export MEOWMAIL_OIDC_REDIRECT_URL='https://mail.example.com/api/v1/auth/oidc/callback'
export MEOWMAIL_OIDC_FIRST_USER_ADMIN=true
./target/release/meowmail
```

OIDC 使用 Authorization Code、PKCE、state 与 nonce，并校验 ID Token 的 issuer、audience、签名和有效期。`MEOWMAIL_OIDC_SCOPES` 默认是 `openid profile email`。混合模式将 `MEOWMAIL_AUTH_MODE` 设置为 `hybrid`，同时配置本地管理员与 OIDC 即可。

如果启用本地登录，首次启动必须同时设置 `MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME` 和 `MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD`。环境变量只负责创建初始管理员，不会覆盖已有用户的密码。

## Docker

正式版本同时发布 `linux/amd64` 与 `linux/arm64` 镜像：

```bash
docker pull ghcr.io/ca-x/meowmail:0.2.0
docker pull czyt/meowmail:0.2.0
```

正式 tag 会生成 `v0.2.0`、`0.2.0`、`0.2`、`latest` 和 `sha-<commit>` 标签。下面以 GHCR 为例：

```bash
docker volume create meowmail-data
docker run --detach \
  --name meowmail \
  --restart unless-stopped \
  --publish 8080:8080 \
  --env MEOWMAIL_AUTH_MODE=local \
  --env MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME=admin \
  --env MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD='请换成足够长的随机密码' \
  --volume meowmail-data:/data \
  ghcr.io/ca-x/meowmail:0.2.0
```

本地构建可把最后一个镜像名换成 `meowmail:local`：

```bash
docker build --tag meowmail:local .
```

通用 Docker 镜像以非 root 用户 `10001:10001` 运行，持久数据写入 `/data`。建议在 Meowmail 前放置提供 HTTPS 的反向代理；HTTPS 环境下登录 Cookie 会根据 `X-Forwarded-Proto: https` 设置 `Secure`。

## 环境变量

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `MEOWMAIL_BIND` | `0.0.0.0:8080` | 监听地址 |
| `MEOWMAIL_DATA_DIR` | `data` | SQLite 与凭据密钥目录 |
| `MEOWMAIL_AUTH_MODE` | `local` | `local`、`oidc` 或 `hybrid` |
| `MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME` | 无 | 首次创建本地管理员的用户名 |
| `MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD` | 无 | 首次创建本地管理员的密码，至少 8 个字符 |
| `MEOWMAIL_VAULT_KEY` | 无 | 可选的固定凭据加密密钥；省略时生成 `vault.key` |
| `MEOWMAIL_OIDC_ISSUER` | 无 | OIDC issuer，生产环境必须是 HTTPS |
| `MEOWMAIL_OIDC_CLIENT_ID` | 无 | OIDC Client ID |
| `MEOWMAIL_OIDC_CLIENT_SECRET` | 无 | OIDC Client Secret；公共客户端可省略 |
| `MEOWMAIL_OIDC_REDIRECT_URL` | 无 | 完整回调 URL |
| `MEOWMAIL_OIDC_SCOPES` | `openid profile email` | 空格分隔的 scopes，必须包含 `openid` |
| `MEOWMAIL_OIDC_FIRST_USER_ADMIN` | `true` | 无管理员时是否把首位 OIDC 用户设为管理员 |

懒猫微服版本会自动使用 `LAZYCAT_AUTH_OIDC_ISSUER_URI`、`LAZYCAT_AUTH_OIDC_CLIENT_ID`、`LAZYCAT_AUTH_OIDC_CLIENT_SECRET` 与 `LAZYCAT_PUBLIC_URL` 作为回退配置，并仅启用 OIDC 登录。

## 邮件账户与代理

界面提供 Gmail、Outlook 和自定义预设。请确认服务商已启用 IMAP/SMTP，并优先使用应用专用密码：

- Gmail：IMAP `imap.gmail.com:993` TLS；SMTP `smtp.gmail.com:465` TLS 或 `587` STARTTLS
- Outlook：IMAP `outlook.office365.com:993` TLS；SMTP `smtp.office365.com:587` STARTTLS
- 自定义：只接受 TLS 或 STARTTLS，不会通过明文连接发送凭据

每个账户可以选择：

- `direct`：直接连接邮件服务器
- `http`：通过 HTTP `CONNECT` 建立 TCP 隧道，可选 Basic 代理认证
- `socks5`：通过 SOCKS5 建立隧道，可选用户名/密码认证

同一邮件账户的 IMAP 与 SMTP 使用同一套代理设置，不同账户互不影响。

## 邮件保留与自动清理

个人设置中可以选择：服务器上不存在的邮件是否继续保留本地副本。自动清理规则可组合以下条件：

- 指定邮件账户
- 发件人包含文本
- 主题包含文本
- 正文包含文本
- 邮件早于指定天数

规则默认只清理本地数据。只有显式启用“同时从服务器删除”时，Meowmail 才会使用邮件 UID 删除服务器副本。

## 新邮件通知

在“设置 → 通知”中可同时配置命令和 HTTP 地址。仅新同步写入 SQLite 的邮件触发通知；通知失败不会导致邮件同步失败。

| 占位符 | 内容 |
| --- | --- |
| `{account}` | 邮件账户显示名 |
| `{email}` | 当前邮件账户地址 |
| `{sender}` | 发件人显示名 |
| `{sender_email}` | 发件人邮箱地址 |
| `{subject}` | 邮件主题 |
| `{preview}` | 邮件摘要 |
| `{message}` | 已渲染的消息模板，仅命令参数可用 |

消息模板示例：

```text
[{account}] {sender}: {subject}
```

命令模板示例：

```text
/usr/bin/notify-send "{account}" "{message}"
```

安全边界：可执行文件必须是固定绝对路径且不能包含占位符；参数解析后才替换邮件字段；程序直接执行 argv、不调用 shell；Webhook 必须是固定的 `http://` 或 `https://` 地址，不允许 URL 凭据和模板，也不会跟随重定向。

Webhook 使用 `POST` JSON：

```json
{
  "message": "[Work] Alice: Project update",
  "account": "Work",
  "email": "me@example.com",
  "sender": "Alice",
  "senderEmail": "alice@example.com",
  "subject": "Project update",
  "preview": "The latest status is..."
}
```

## 配置导入与导出

归档使用用户提供的口令，经 Argon2id 派生密钥后用 XChaCha20-Poly1305 加密。导出时可以独立选择：

- 资料与头像
- 邮件账户、代理与邮件凭据
- 推送设置
- 邮件保留选项与自动清理规则

管理员默认导出“仅我的配置”，也可以明确选择“所有用户”。全用户归档包含恢复账号所需的角色、密码/PIN 哈希及 OIDC issuer/subject，但不包含明文登录密码、OIDC Token、Client Secret 或会话。普通用户无法创建或导入全用户归档。

## 安全与备份

- 用户密码和 PIN 使用 Argon2id 哈希；邮箱与代理密码使用 XChaCha20-Poly1305 加密
- 会话保存在服务端内存，Cookie 为 HttpOnly、SameSite=Lax；服务重启后需要重新登录
- 所有写 API 使用 CSRF token，登录失败有节流限制
- HTML 邮件在后端清洗后放入 sandboxed iframe，默认阻止远程图片
- API 有请求体大小限制、安全响应头与泛化的外部服务错误
- SQLite 使用 WAL、外键和 busy timeout

停止容器或进程后备份完整数据目录：

```text
data/
├── meowmail.sqlite3
└── vault.key       # 未设置 MEOWMAIL_VAULT_KEY 时
```

如果设置了 `MEOWMAIL_VAULT_KEY`，数据目录中会保存 `vault.salt`，恢复时还必须提供原来的环境变量值。无论采用哪种方式，只备份 SQLite 都不足以恢复邮件账户凭据。

## 开发与验证

```bash
make dev       # Rust API + Vite 开发服务器
make web       # 锁定依赖并构建 Web 资源
make test      # Rust fmt/clippy/test + Web typecheck/test
make build     # Web + release 嵌入式单文件
make docker    # meowmail:local 镜像
```

完整发布前验证：

```bash
npm --prefix web ci --ignore-scripts
npm --prefix web audit --audit-level=high
npm --prefix web audit signatures
npm --prefix web run typecheck
npm --prefix web run test:ci
npm --prefix web run build
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked
```

推送 `v0.2.0` tag 后：

- `.github/workflows/release.yml` 构建 Linux x86_64/aarch64、Windows x86_64、macOS x86_64/aarch64 压缩包，生成 `SHA256SUMS` 并发布 GitHub Release。
- `.github/workflows/docker.yml` 构建 amd64/arm64 镜像，附带 provenance 与 SBOM，并同时发布到 `ghcr.io/ca-x/meowmail` 与 `czyt/meowmail`。
- Docker 工作流支持用严格的 `refs/tags/<release_tag>` 手动补发已有版本；仓库或组织需要提供 `DOCKERHUB_USERNAME` 与 `DOCKERHUB_TOKEN`。

## License

MIT
