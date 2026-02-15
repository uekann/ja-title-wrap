#import "../src/lib.typ": auto-title

#set page(width: 254mm, height: 142.9mm, margin: 10mm)
#set text(font: "Source Han Sans JP", size: 16pt)

#let title-slide(title) = {
  align(center + horizon)[
    #text(size: 30pt, weight: "black", title)
  ]
  pagebreak()
}

#let samples = (
  "形態素解析ベースで長いタイトルを自然に自動改行する",
  "短いタイトル",
  "Typst plugin で日本語タイトルの見た目を安定化する",
  "句読点、括弧（かっこ）を含むタイトルの禁則確認",
  "プログラミング学習者向けの問題解決の事例検索サービス"
)

#for sample in samples [
  #title-slide(auto-title(
    sample,
    max-width: 200mm,
    max-lines: 2,
  ))
]
