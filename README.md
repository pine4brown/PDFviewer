# WaffleMatrix (軽量PDFビューアー)

WaffleMatrix は Tauri と Vanilla JavaScript で構築された、高速で軽量なデスクトップ向け PDF ビューアーです。

## 前提条件 (Prerequisites)

WaffleMatrix をビルドおよび実行するには、以下の環境が必要です。

### 全プラットフォーム共通
- **Node.js** (v18以降推奨) および **npm**
- **Rust** (最新の安定版)
  - [rustup](https://rustup.rs/) を使用してインストールしてください。

### macOS でのセットアップ
macOS で開発・ビルドを行う場合、Cコンパイラなどのビルドツールが必要です。ターミナルを開き、以下のコマンドを実行して Xcode Command Line Tools をインストールしてください。

```bash
xcode-select --install
```

### Windows でのセットアップ
Windows で開発・ビルドを行う場合、以下のツールが必須となります。

1. **Microsoft Visual Studio C++ Build Tools**: 
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/ja/visual-cpp-build-tools/) をダウンロードし、インストーラーから **「C++ によるデスクトップ開発」** ワークロードを選択してインストールしてください。
2. **WebView2**: 
   - Windows 11 の場合は標準で組み込まれていますが、それ以前の環境では [WebView2 ランタイム](https://developer.microsoft.com/ja-jp/microsoft-edge/webview2/) のインストールが必要です。

---

## インストール手順 (Installation)

1. 当リポジトリをクローンし、プロジェクトのディレクトリに移動します。
```bash
git clone https://github.com/pine4brown/PDFviewer.git
cd PDFviewer
```

2. 必要な npm パッケージをインストールします。
```bash
npm install
```

## 実行およびビルド (Usage)

### 開発モードで実行する (Development)
コードの変更を即座に反映（ホットリロード）できる開発モードでアプリを起動します。
```bash
npx tauri dev
```

### プロダクション向けにビルドする (Build)
本番用の実行ファイルやインストーラーを生成するには、以下のコマンドを実行します。
```bash
npx tauri build
```
ビルドが完了すると、OSごとに以下のディレクトリにアプリケーションが生成されます。
- **macOS**: `src-tauri/target/release/bundle/macos/` もしくは `dmg/`
- **Windows**: `src-tauri/target/release/bundle/msi/` もしくは `nsis/`

---

## 特徴
- Tauri の採用による、ネイティブに近いパフォーマンスと軽量なフットプリント
- Vanilla Web テクノロジーを使用したシンプルで拡張しやすい UI

## ライセンス
LICENSE ファイルを参照してください。
