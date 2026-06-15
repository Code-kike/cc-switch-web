<div align="center">

# cc-switch-web

### Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw のための Web ファースト・リモート管理ツール

[![Platform](https://img.shields.io/badge/platform-Linux%20Server%20%7C%20Browser-lightgrey.svg)](#現在のデプロイモデル)
[![Built with Tauri](https://img.shields.io/badge/backend-Tauri%202%20Web%20Server-orange.svg)](https://tauri.app/)
[![Frontend](https://img.shields.io/badge/frontend-React%20%2B%20Vite-646cff.svg)](#開発)

[English](README.md) | [中文](README_ZH.md) | 日本語

</div>

## 概要

`cc-switch-web` は `cc-switch` エコシステムの Web ファースト（web-first）なデプロイ形態であり、ローカルの AI CLI ツール設定に対する**リモートアクセス**、**常駐サーバーデプロイ**、**ブラウザベースの管理**に重点を置いています。

頻繁にリモートからアクセスするマシン上で Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw を管理している場合、本プロジェクトはローカルのデスクトップアプリを必要とせず、Web UI による管理を提供します。

## 謝辞

本プロジェクトは 2 つのアップストリームプロジェクトから直接恩恵を受けています：

- [`farion1231/cc-switch`](https://github.com/farion1231/cc-switch)：成熟した製品基盤、データモデル、プロバイダ管理ロジック、マルチツール統合、コアとなるバックエンド機能を提供しています。
- [`Laliet/CC-Switch-Web`](https://github.com/Laliet/CC-Switch-Web)：ブラウザベースの方向性を示し、`cc-switch` ワークフローにおけるリモート Web 管理の価値を実証しました。

`cc-switch-web` は、これら 2 つの流れを実用的に統合したものです。`cc-switch` の新しい機能に追従しつつ、それらをリモートからアクセス可能な Web デプロイモデルを通じて利用できるようにします。

## なぜこのプロジェクトなのか

オリジナルの `cc-switch` デスクトップアプリはローカル利用には強力ですが、以下のような場面では理想的ではありません：

- メインマシンに SSH やリモートデスクトップ経由でアクセスしている
- 再起動後もサービスをオンラインのままにしておきたい
- LAN 上の別のデバイスからブラウザでアクセスしたい
- デスクトップ GUI を起動せずにプロバイダ、プロンプト、MCP、スキル、セッションを管理したい

本リポジトリはこのギャップの解決に焦点を当てています。

## 提供する機能

- **Web ファーストの管理 UI** -- Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw に対応
- **リモートブラウザアクセス** -- セルフホストのマシン上で
- **Systemd サービスデプロイ** -- 常時稼働と再起動時の自動起動
- **既存の `~/.cc-switch` データの再利用** -- 別個のデータサイロを強制しない
- **モダンな `cc-switch` 機能ベース** -- 初期の Web プロトタイプに留まらない
- **スタンドアロン Web サーバーランタイム** -- Linux サーバーまたはワークステーションへのデプロイ向け

Web サーバーモードは主にリモートでの設定・管理を目的としています。プロバイダ、プロンプト、MCP
サーバー、スキル、セッション、および関連設定をブラウザから編集できますが、ローカルのプロキシ
ランタイムを制御する操作は引き続きデスクトップ専用です。Web モードでは、プロキシ（Proxy）と
フェイルオーバー（Failover）の設定は構成として編集できますが、ローカルプロキシプロセスの起動、
ランタイムのテイクオーバー、ライブプロキシ制御はスタンドアロンサーバーからは意図的に公開されません。

## 現在のデプロイモデル

本プロジェクトは現在、セルフホストの Linux 利用に最適化されています。

典型的なデプロイ手順：

1. `pnpm build:web` で Web フロントエンドをビルド
2. Cargo でスタンドアロンサーバーをビルド
3. バイナリと静的アセットをインストール
4. `systemd --user` サービスとして実行
5. `http://<host>:3010` からアクセス

本リポジトリでは、サービスデプロイは以下を既にサポートしています：

- バインドアドレス `0.0.0.0`
- デフォルトポート `3010`
- `systemd --user` による自動起動
- 静的アセットを `~/.local/share/cc-switch-web/dist-web` へインストール
- `~/.cc-switch` のデータ再利用

> **非ループバックバインドには認証が必要：** 非ループバックバインド（`0.0.0.0`、
> LAN IP、または Tailscale アドレス）では HTTP Basic 認証が**必須**です。
> `CC_SWITCH_WEB_AUTH_PASSWORD`（任意で `CC_SWITCH_WEB_AUTH_USER`、デフォルトは
> `cc-switch`）を設定してください。パスワード未設定の場合、サーバーは非ループバック
> アドレスでの起動を拒否します。インストールスクリプトは `0600` 権限の systemd
> パスワード drop-in を自動生成します。ループバックのみ（`127.0.0.1`）のローカル
> 開発実行ではパスワードは不要です。

## リポジトリ構成

- `src/`：React + Vite フロントエンド
- `src-tauri/`：共有バックエンドロジックとスタンドアロン Web サーバー
- `deploy/systemd/`：ユーザーサービスユニット
- `scripts/install-cc-switch-web-service.sh`：常駐サービスデプロイ用のビルド & インストールスクリプト
- `dist-web/`：生成された Web フロントエンドのビルド成果物

## 開発

### フロントエンド開発

```bash
pnpm install
pnpm dev:web
```

### Web ビルド

```bash
pnpm build:web
```

### スタンドアロン Web サーバー

```bash
cargo run --manifest-path src-tauri/Cargo.toml \
  --no-default-features \
  --features web-server \
  --example server
```

### サービスインストール

```bash
./scripts/install-cc-switch-web-service.sh
```

### サービス管理

```bash
systemctl --user status cc-switch-web.service --no-pager
systemctl --user restart cc-switch-web.service
journalctl --user -u cc-switch-web.service -f
```

## データディレクトリ

現在のサービスデプロイは以下を再利用するよう構成されています：

```bash
~/.cc-switch
```

つまり、既存のプロバイダ、プロンプト、スキル、バックアップ、および関連データは、再起動時にリセットされたり、デフォルトで 2 つ目のデータベースに分割されたりすることなく、引き続き Web サービスで利用できます。

## プロジェクトの位置づけ

本リポジトリは、概念的にアップストリームプロジェクトを置き換えようとするものではありません。

その役割はより限定的で、より実用的です：

- `cc-switch` の新しい機能に追従する
- それをリモートで利用可能な Web UI を通じて公開する
- 長時間稼働するセルフホストデプロイをサポートする
- デスクトップ志向の `cc-switch` と初期の Web 志向プロトタイプとの間のギャップを縮める

## ステータス

本プロジェクトには既に動作するスタンドアロン Web ランタイムと永続的なサービスデプロイ経路がありますが、一部の領域では機能パリティの作業が継続中です。主な方向性は、`cc-switch` の新しい機能を Web 体験に同期し続け、残された管理上のギャップを埋めることです。

## アップストリームプロジェクト

- `cc-switch`: https://github.com/farion1231/cc-switch
- `CC-Switch-Web`: https://github.com/Laliet/CC-Switch-Web

## ライセンス

本リポジトリは現在、このプロジェクトツリーに含まれるライセンス条項に従います。再配布または派生利用の前に `LICENSE` ファイルをご確認ください。
