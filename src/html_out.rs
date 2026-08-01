use base64::Engine;
use crate::utils::CHAR_WIDTH_RATIO;

// The exact Consolas font file claytons-philosophy-notes self-hosts (see its
// app/layout.js, which loads this via next/font/local) - CHAR_WIDTH_RATIO
// was measured specifically against this font's glyph metrics, so without
// embedding this same file, a browser without Consolas installed silently
// falls back to a different monospace font and the whole grid misaligns
// (rows don't fill the width, gaps appear between rows). include_bytes!
// compiles the font straight into this binary, so no external file is
// needed at runtime - the generated HTML embeds it as a data URI below.
const CONSOLAS_WOFF2: &[u8] = include_bytes!("../assets/consolas.woff2");

// Converts an xterm-256 palette index (16-231, the 6x6x6 color cube) back
// into an actual (r, g, b) color a browser can render - this is the exact
// reverse of the "quantize" math in pick_colors, and mirrors paletteColor()
// in claytons-philosophy-notes/lib/ansi.js (same formula, opposite direction).
fn palette_index_to_rgb(index: u8) -> (u8, u8, u8) {
    // undo the "+16" offset from pick_colors to get back to a 0-215 cube position
    let code = (index as u32) - 16;

    // undo the flattening: pull r_step/g_step/b_step back out of one combined number
    let r_step = code / 36;
    let g_step = (code % 36) / 6;
    let b_step = code % 6;

    // undo the "/51" step-rounding to get back to a real 0-255 channel value
    ((r_step * 51) as u8, (g_step * 51) as u8, (b_step * 51) as u8)
}

// Writes a minimal, self-contained HTML file that renders the ANSI grid the
// same way claytons-philosophy-notes does it: one <div class="ansi-line">
// per row, one <span> per cell colored via inline style, all inside a <pre>
// so spacing is preserved exactly. The CSS below is copied straight out of
// that site's app/globals.css (.ansi-frame/.ansi-pre/.ansi-line rules), so
// this file renders the same without needing the rest of that site.
pub fn write_html_output(colors: &Vec<Vec<(u8, char)>>, cols: usize, path: &str) {
    // same character-cell aspect-ratio number used earlier to pick `rows` -
    // now used to size the font so `cols` characters exactly fill the page
    // width, matching AnsiArt.js's `effectiveCols`.
    let effective_cols = (cols as f32) * CHAR_WIDTH_RATIO;

    // turn the embedded font bytes into a base64 data: URI, so the <style>
    // block below can reference it directly with no separate font file
    let font_base64 = base64::engine::general_purpose::STANDARD.encode(CONSOLAS_WOFF2);

    // build the <div class="ansi-line">...</div> markup, one per grid row
    let mut rows_html = String::new();
    for row in colors.iter() {
        let mut row_html = String::new();
        for &(palette_index, glyph) in row {
            let (r, g, b) = palette_index_to_rgb(palette_index);
            row_html.push_str(&format!(
                "<span style=\"color: rgb({r},{g},{b}); background-color: rgb({r},{g},{b});\">{glyph}</span>"
            ));
        }
        // no trailing \n here - .ansi-pre uses white-space: pre, which
        // renders literal newline characters as actual blank lines. The
        // <div>s are already block-level so they stack on their own line
        // regardless; an extra \n between them just adds a visible gap.
        rows_html.push_str(&format!("<div class=\"ansi-line\">{row_html}</div>"));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>ANSI Art</title>
<style>
  @font-face {{
    font-family: "ConsolasEmbedded";
    src: url(data:font/woff2;base64,{font_base64}) format("woff2");
    font-weight: 400;
    font-style: normal;
  }}
  body {{ background: #000; margin: 0; padding: 2rem; }}
  .ansi-frame {{
    container-type: inline-size;
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
    background: #000;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
    cursor: default;
  }}
  .ansi-pre {{
    margin: 0;
    padding: 0;
    line-height: 1;
    letter-spacing: 0;
    white-space: pre;
    width: max-content;
    font-family: "ConsolasEmbedded", Consolas, monospace;
  }}
  .ansi-line {{
    height: 1em;
    overflow: hidden;
    display: flex;
  }}
  .ansi-line span {{
    display: inline-block;
  }}
</style>
</head>
<body>
  <div class="ansi-frame">
    <pre class="ansi-pre" style="font-size: clamp(4px, calc(100cqw / {effective_cols}), 16px);">
{rows_html}    </pre>
  </div>
</body>
</html>
"#
    );

    std::fs::write(path, html).unwrap();
    println!("wrote HTML output to {}", path);
}
