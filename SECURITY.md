# Security Policy / 安全策略

## Supported Versions / 支持的版本

Only the latest release of CC Switch receives security updates.

仅最新版本的 CC Switch 会收到安全更新。

| Version / 版本 | Supported / 是否支持 |
|----------------|---------------------|
| Latest 3.x     | ✅ Yes / 是          |
| < 3.0          | ❌ No / 否           |

## Reporting a Vulnerability / 报告漏洞

**Please do NOT report security vulnerabilities through public GitHub issues.**

**请不要通过公开的 GitHub Issue 报告安全漏洞。**

Instead, please report them through [GitHub Security Advisories](https://github.com/farion1231/cc-switch/security/advisories/new).

请通过 [GitHub 安全公告](https://github.com/farion1231/cc-switch/security/advisories/new) 进行报告。

When reporting, please include:

报告时请包含以下信息：

- A description of the vulnerability / 漏洞描述
- Steps to reproduce / 复现步骤
- Potential impact / 潜在影响
- Affected versions / 受影响版本

## Response Timeline / 响应时间

- **Acknowledgment / 确认**: within 48 hours / 48 小时内
- **Initial assessment / 初步评估**: within 7 days / 7 天内
- **Fix for critical issues / 关键问题修复**: within 14 days / 14 天内

## Disclosure Policy / 披露政策

We follow a coordinated disclosure process:

我们遵循协调披露流程：

1. The reporter submits the vulnerability privately. / 报告者私下提交漏洞。
2. We confirm and work on a fix. / 我们确认并修复漏洞。
3. A patch release is published. / 发布修复版本。
4. The vulnerability is publicly disclosed. / 公开披露漏洞详情。

Reporters will be credited in the release notes unless they prefer to remain anonymous.

除非报告者希望匿名，否则将在发布说明中致谢。

## Security Updates / 安全更新

Security fixes are released as patch versions and announced via [GitHub Releases](https://github.com/farion1231/cc-switch/releases). We recommend always updating to the latest version.

安全修复通过补丁版本发布，并通过 [GitHub Releases](https://github.com/farion1231/cc-switch/releases) 通知。建议始终更新到最新版本。

## Web Deployment: Sensitive Files at Rest / Web 部署：静态敏感文件

When running `cc-switch-web` as an always-on service, the service account's data
directory may contain long-lived credentials, notably:

在以常驻服务方式运行 `cc-switch-web` 时，服务账号的数据目录可能包含长期有效的凭据，尤其是：

- `codex_oauth_auth.json` — Codex/ChatGPT OAuth refresh tokens (`0600` on Unix).
- Copilot OAuth auth store (same class).
- CLI config files under `~/.claude`, `~/.codex`, `~/.gemini` that hold API keys.

These files are written with `0600` permissions but are **not encrypted at rest**.
The built-in WebDAV / S3 sync snapshot bundles only the SQLite database and the
skills archive — it does **not** upload these OAuth files. If you run your own
backups (rsync, snapshots, external sync), exclude these files or ensure the
backup target is trusted, since anyone who can read them obtains long-lived tokens.

这些文件以 `0600` 权限写入，但**未加密存储**。内置的 WebDAV / S3 同步快照仅打包 SQLite
数据库与技能归档，**不会**上传这些 OAuth 文件。若你使用自有备份方案（rsync、快照、外部同步），
请排除上述文件或确保备份目标可信——任何能读取它们的人都会获得长期有效的令牌。
