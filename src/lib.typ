#let _analyzer = plugin("/plugin/ja_title_wrap_core.wasm")

#let _plugin-analyze(text) = {
  let raw = _analyzer.analyze_ja_title(bytes(text))
  json(raw)
}

#let _normalize-analysis(data) = {
  assert(type(data) == dictionary, message: "auto-title plugin returned invalid JSON")
  let tokens = data.at("tokens")
  let break-after = data.at("break_after")
  assert(type(tokens) == array, message: "auto-title plugin: tokens must be array")
  assert(type(break-after) == array, message: "auto-title plugin: break_after must be array")

  (
    tokens: tokens,
    break_after: break-after,
  )
}

#let _build-line(tokens, start, end) = {
  if end <= start {
    return ""
  }
  tokens.slice(start, end).join("")
}

#let _compare-score(a, b) = {
  if b == none {
    return true
  }
  if a.kinsoku != b.kinsoku {
    return a.kinsoku < b.kinsoku
  }
  if a.overflow != b.overflow {
    return a.overflow < b.overflow
  }
  if a.short != b.short {
    return a.short < b.short
  }
  a.balance < b.balance
}

#let _valid-breaks(tokens, raw-breaks) = {
  if tokens.len() < 2 {
    return ()
  }

  let valid = ()
  let max-index = tokens.len() - 1
  for raw in raw-breaks {
    if type(raw) != int {
      continue
    }
    if raw < 0 or raw >= max-index {
      continue
    }
    if raw in valid {
      continue
    }
    valid.push(raw)
  }

  valid
}

#let _best-two-lines(tokens, breaks, max-width, min-ratio, line-width) = {
  let best = none

  for k in breaks {
    let line1 = _build-line(tokens, 0, k + 1)
    let line2 = _build-line(tokens, k + 1, tokens.len())
    if line1 == "" or line2 == "" {
      continue
    }

    let w1 = line-width(line1)
    let w2 = line-width(line2)
    let overflow = (if w1 > max-width { 1 } else { 0 }) + (if w2 > max-width { 1 } else { 0 })
    let short = if calc.min(w1, w2) < max-width * min-ratio { 1 } else { 0 }
    let balance = calc.abs(w1 - w2)

    let score = (
      kinsoku: 0,
      overflow: overflow,
      short: short,
      balance: balance,
      text: line1 + "\n" + line2,
    )
    if _compare-score(score, best) {
      best = score
    }
  }

  best
}

#let _best-three-lines(tokens, breaks, max-width, min-ratio, line-width) = {
  let best = none
  for k1 in breaks {
    for k2 in breaks {
      if k2 <= k1 {
        continue
      }

      let line1 = _build-line(tokens, 0, k1 + 1)
      let line2 = _build-line(tokens, k1 + 1, k2 + 1)
      let line3 = _build-line(tokens, k2 + 1, tokens.len())
      if line1 == "" or line2 == "" or line3 == "" {
        continue
      }

      let w1 = line-width(line1)
      let w2 = line-width(line2)
      let w3 = line-width(line3)

      let overflow = (
        if w1 > max-width { 1 } else { 0 }
      ) + (
        if w2 > max-width { 1 } else { 0 }
      ) + (
        if w3 > max-width { 1 } else { 0 }
      )

      let min12 = if w1 < w2 { w1 } else { w2 }
      let min-width = if min12 < w3 { min12 } else { w3 }
      let max12 = if w1 > w2 { w1 } else { w2 }
      let max-width-local = if max12 > w3 { max12 } else { w3 }

      let short = if min-width < max-width * min-ratio { 1 } else { 0 }
      let balance = max-width-local - min-width

      let score = (
        kinsoku: 0,
        overflow: overflow,
        short: short,
        balance: balance,
        text: line1 + "\n" + line2 + "\n" + line3,
      )
      if _compare-score(score, best) {
        best = score
      }
    }
  }

  best
}

#let auto-title(
  text,
  max-width: 76%,
  max-lines: 2,
  min-ratio: 0.38,
) = context {
  if max-lines <= 1 {
    return text
  }

  let one-line-width = measure([#text]).width
  if one-line-width <= max-width {
    return text
  }

  let analysis = _normalize-analysis(_plugin-analyze(text))

  let tokens = analysis.tokens
  if tokens.len() <= 1 {
    return text
  }

  let breaks = _valid-breaks(
    tokens,
    analysis.break_after,
  )
  if breaks.len() == 0 {
    return text
  }

  let line-width = line => measure([#line]).width
  let two = _best-two-lines(tokens, breaks, max-width, min-ratio, line-width)
  if max-lines == 2 or two == none {
    return if two == none { text } else { two.text }
  }

  let three = _best-three-lines(tokens, breaks, max-width, min-ratio, line-width)
  if three == none {
    return two.text
  }

  if _compare-score(three, two) {
    three.text
  } else {
    two.text
  }
}
