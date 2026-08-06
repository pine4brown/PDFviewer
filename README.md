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
  bench/
    case.rs     → ground truth データモデル
    gen.rs      → 合成PDF生成（決定的・シード付き）
    score.rs    → 精度メトリクス（P/R/F1）
    eval.rs     → コーパス実行・集計・レポート
```

## CLIツール（精度評価）

GUIを使わずヘッドレスで差分を取れる **`wafflematrix-cli`** を提供しています。
`src-tauri` ディレクトリから実行します（PDFiumは自動検出。`--pdfium <path>` または環境変数 `PDFIUM_LIB_PATH` で上書き可）。

```bash
# 2つのPDFを比較し、人間向けサマリを表示
cargo run --bin wafflematrix-cli -- compare --old a.pdf --new b.pdf --mode hybrid

# レポートをJSON/Excel等に出力（拡張子で形式決定。 "-" はJSONをstdoutへ）
cargo run --bin wafflematrix-cli -- compare --old a.pdf --new b.pdf --output report.xlsx

# 合成テストコーパスを再生成（testdata/corpus/）
cargo run --bin wafflematrix-cli -- gen --force

# 精度評価を実行（--min-f1/--min-visual-f1未達でexit 1、CIゲートに使用）
cargo run --bin wafflematrix-cli -- eval --min-f1 0.9 --min-visual-f1 0.85

# 実PDFペアを"ゴールデン"ケースとして凍結（現在の出力をground truth化して回帰監視）
cargo run --bin wafflematrix-cli -- golden --name sample_docs --old v1.pdf --new v2.pdf
```

### 評価コーパス（`testdata/corpus/`）
各ケースは `<name>/old.pdf` `<name>/new.pdf` `<name>/ground_truth.json` で構成されます。
`eval` は各ケースを各モードで比較し、ground truth と照合して以下を報告します。

- **テキスト内容一致F1**（主指標・text/hybrid）: 変更行を正規化テキストで突合。追加/削除/修正の分類差に寛容
- **領域重なりF1**（visual）: 検出`visual_rects`とGT矩形を包含率で突合
- **ページ分類精度**: ページ状態（match/modified/added/removed）の一致率
- **誤検知ドキュメント数**: 全ページが変化なしと期待されるケースで差分を誤検出した数

visual比較のアライメントは、まず**テキスト行座標**（PDFium抽出）でオフセットを推定し、
テキストの無いページでは**位相相関＋MAD精緻化**へフォールバックします。これにより
「図が移動した」ケースで無変更テキストを2重写しにしない頑健な整列と、
サブピクセルシフト（2pt並進等）の相殺が両立します。現在の合成コーパス12件の
visualモードは 領域F1=1.000・ページ分類精度=1.000・誤検知0 です（`eval --modes visual`）。

`gen` の合成ケースは `src-tauri/src/bench/gen.rs` の `case_definitions()` に定義されており、
シードから決定的に生成されます。**新しいテストケースの追加**は、同関数にケースを足し、
`gen --force` で再生成するだけです。実PDFの回帰監視には `golden` サブコマンドを使います。

### CI
GitHub Actions（`.github/workflows/ci.yml`）がpush/PR時に
`cargo build` → `cargo test` → `gen --force` → `eval --min-f1 0.9 --min-visual-f1 0.85` を実行し、
テキスト差分精度と図表・図形の領域検出精度のリグレッションを自動検出します。

## ライセンス
LICENSE ファイルを参照してください。
