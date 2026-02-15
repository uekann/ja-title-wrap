# ja-title-wrap

Typst の日本語タイトル自動改行プラグインです。  
改行候補は Rust/Wasm 側（Lindera + IPADIC）で作り、最終的な改行位置は Typst 側で `measure` を使って決めます。  

## ディレクトリ構成

- `src/`: Typst パッケージ本体
- `plugin/`: 配布用 Wasm バイナリ配置先
- `ja-title-wrap-core/`: Rust 実装（開発用）
- `examples/`: サンプル

## Typst パッケージ情報

- Manifest: `typst.toml`
- Entrypoint: `src/lib.typ`

## ビルド

```bash
just build-plugin
```

## テスト

```bash
just test
```

## このリポジトリ内での利用

```typst
#import "src/lib.typ": auto-title

#auto-title(
  "形態素解析ベースで長いタイトルを自然に自動改行する",
  max-width: 120mm,
  max-lines: 2,
)
```

## デモ

```bash
just demo
```
