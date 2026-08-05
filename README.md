# WaffleMatrix (軽量PDFビューアー / PDF差分ツール)

WaffleMatrix は Tauri と Vanilla JavaScript で構築された、高速で軽量なデスクトップ向け PDF ビューアーです。2つのPDFを比較して差分を表示・出力する PDF差分ツール機能も備えています。

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

## PDF差分比較機能

ツールバーの **Compare / 比較** ボタンから、2つのPDFファイルの差分を確認できます。

### 比較モード
- **Text（テキスト）**: データシート・仕様書向け。PDFiumで抽出したテキストを座標付きで読み順に整列し、行単位の差分（追加/削除/変更）を検出します。
- **Visual（ビジュアル）**: 回路図・レイアウト向け。ページを300DPIでラスタライズし、軽量アライメント（サブピクセルずれの吸収）を行った上で画素差分を検出し、変更領域を報告します。
- **Hybrid（ハイブリッド）**: テキスト差分に加え、テキスト変更で検出されないビジュアル領域も併せて報告します。

### 出力形式
比較結果は以下の形式でエクスポートできます（**Excel出力**がメイン）。

| 形式 | 拡張子 | 内容 |
| --- | --- | --- |
| Excel | `.xlsx` | 「変更一覧」（ページ・変更種別・変更前後テキスト・座標）、「ページサマリー」、「概要」の3シート。変更種別ごとにセル色分け |
| CSV | `.csv` | 変更一覧と同列のフラットな表 |
| JSON | `.json` | 比較レポート全体（フロント表示と同一データ） |
| HTML | `.html` | 変更ページごとのサイドバイサイド表示レポート |

### バックエンド構成
```
src-tauri/src/
  diff/
    loader.rs   → 2ファイル読込・ページ対応付け
    text.rs     → 座標付きテキスト抽出・読書順クラスタリング
    diff.rs     → similarによる行単位Myers差分
    visual.rs   → ラスタライズ・軽量アライメント・画素比較
    report.rs   → DiffReport データモデル
    export.rs   → xlsx / csv / json / html 出力
```

## ライセンス
LICENSE ファイルを参照してください。
