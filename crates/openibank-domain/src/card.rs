//! Receipt card renderers — ASCII, SVG, and HTML.
//!
//! These are marketing materials designed to be screenshot and shared.
//! The SVG card includes a QR code linking to the verify URL.

use std::path::{Path, PathBuf};
use crate::{DomainError, Receipt};

/// Which card formats to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardFormat { Ascii, Svg, Html, Both, All }

// ── ASCII Card ────────────────────────────────────────────────────────────────

pub fn render_ascii(receipt: &Receipt) -> String {
    let verified = if receipt.receipt_sig.is_empty() { "UNSIGNED" } else { "VERIFIED ✓" };
    let bar = "─".repeat(55);
    let mut out = String::new();
    out.push_str(&format!("┌{}┐\n", bar));
    out.push_str(&format!("│  {} OPENIBANK RECEIPT{}\n", verified, " ".repeat(55usize.saturating_sub(verified.len() + 20))));
    out.push_str(&format!("├{}┤\n", bar));
    let push = |out: &mut String, label: &str, val: &str| {
        let s = format!("│  {:10} {}", label, shorten(val, 42));
        out.push_str(&format!("{}{}\n", s, " ".repeat(57usize.saturating_sub(s.chars().count()))));
    };
    let mut buf = String::new();
    push(&mut buf, "ID:", &receipt.tx_id);
    push(&mut buf, "FROM:", &receipt.from.0);
    push(&mut buf, "TO:", &receipt.to.0);
    push(&mut buf, "AMOUNT:", &receipt.iusd_amount().to_display_string());
    push(&mut buf, "TYPE:", &format!("{:?}", receipt.action_type));
    out.push_str(&buf);
    out.push_str(&format!("├{}┤\n", bar));
    let mut buf2 = String::new();
    push(&mut buf2, "PERMIT:", &receipt.permit_id);
    push(&mut buf2, "COMMIT:", &receipt.commitment_id);
    push(&mut buf2, "WLL:", &receipt.worldline_pointer());
    if !receipt.receipt_sig.is_empty() {
        push(&mut buf2, "SIG:", &format!("ed25519:{}...", &receipt.receipt_sig[..8.min(receipt.receipt_sig.len())]));
    }
    out.push_str(&buf2);
    out.push_str(&format!("├{}┤\n", bar));
    out.push_str(&format!("│  \"{}\"\n", shorten(&receipt.tagline, 50)));
    out.push_str(&format!("│  Settled: {}\n", receipt.timestamp.format("%Y-%m-%dT%H:%M:%SZ")));
    out.push_str(&format!("│  Verify:  openibank.com/verify/{}\n", shorten(&receipt.tx_id, 30)));
    out.push_str(&format!("└{}┘\n", bar));
    out
}

fn shorten(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}

// ── SVG Card ──────────────────────────────────────────────────────────────────

pub fn render_svg(receipt: &Receipt) -> String {
    // CSS font-family with single quotes — define as const to avoid Rust parser confusion
    // inside format! strings (Rust 2021 treats 'X' prefixes as unknown char literal prefixes).
    const FF: &str = "font-family=\"monospace\"";
    const FF_BOLD: &str = "font-family=\"monospace\"";

    let qr_svg = qr_code_svg(&receipt.verify_url());
    let verified = if receipt.receipt_sig.is_empty() { "UNSIGNED" } else { "VERIFIED" };
    let badge_color = if receipt.receipt_sig.is_empty() { "#ffab40" } else { "#00e676" };
    let amount_str = receipt.iusd_amount().to_display_string();

    let fields = vec![
        format!("ID:     {}", shorten(&receipt.tx_id, 40)),
        format!("FROM:   {}", receipt.from.0),
        format!("TO:     {}", receipt.to.0),
        format!("AMOUNT: {}", amount_str),
        format!("PERMIT: {}", shorten(&receipt.permit_id, 40)),
        format!("COMMIT: {}", shorten(&receipt.commitment_id, 40)),
        format!("WLL:    {}", shorten(&receipt.worldline_pointer(), 40)),
        format!("SIG:    {}", if receipt.receipt_sig.is_empty() { "\u{2014}".into() } else { format!("ed25519:{}...", &receipt.receipt_sig[..8.min(receipt.receipt_sig.len())]) }),
        format!("TIME:   {}", receipt.timestamp.format("%Y-%m-%dT%H:%M:%SZ")),
    ];

    let mut text_nodes = String::new();
    for (i, line) in fields.iter().enumerate() {
        let y = 110 + i as i32 * 20;
        text_nodes.push_str(&format!(
            "  <text x=\"24\" y=\"{}\" {} font-size=\"13\" fill=\"#c8d0e0\">{}</text>\n",
            y, FF, escape_xml(line)
        ));
    }

    let tagline = escape_xml(&shorten(&receipt.tagline, 60));
    let tx_id = shorten(&receipt.tx_id, 32);

    let mut svg = String::with_capacity(4096);
    svg.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"640\" height=\"340\" viewBox=\"0 0 640 340\">\n");
    svg.push_str("  <defs>\n");
    svg.push_str("    <linearGradient id=\"bg\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">\n");
    svg.push_str("      <stop offset=\"0%\" stop-color=\"#0a0e1a\"/>\n");
    svg.push_str("      <stop offset=\"100%\" stop-color=\"#131929\"/>\n");
    svg.push_str("    </linearGradient>\n");
    svg.push_str("  </defs>\n");
    svg.push_str("  <rect width=\"640\" height=\"340\" rx=\"16\" fill=\"url(#bg)\"/>\n");
    svg.push_str("  <rect x=\"1\" y=\"1\" width=\"638\" height=\"338\" rx=\"15\" fill=\"none\" stroke=\"#1e2d45\" stroke-width=\"1\"/>\n");
    svg.push_str("  <rect x=\"0\" y=\"0\" width=\"640\" height=\"72\" rx=\"16\" fill=\"#0f1729\"/>\n");
    svg.push_str("  <rect x=\"0\" y=\"56\" width=\"640\" height=\"16\" fill=\"#0f1729\"/>\n");
    svg.push_str(&format!("  <text x=\"24\" y=\"38\" {} font-size=\"22\" font-weight=\"bold\" fill=\"#00d4ff\">OpeniBank</text>\n", FF_BOLD));
    svg.push_str(&format!("  <text x=\"170\" y=\"38\" {} font-size=\"14\" fill=\"#4a5568\">/ Maple WorldLine</text>\n", FF));
    svg.push_str(&format!("  <rect x=\"520\" y=\"18\" width=\"96\" height=\"28\" rx=\"6\" fill=\"{}\" opacity=\"0.15\"/>\n", badge_color));
    svg.push_str(&format!("  <rect x=\"520\" y=\"18\" width=\"96\" height=\"28\" rx=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>\n", badge_color));
    svg.push_str(&format!("  <text x=\"568\" y=\"37\" {} font-size=\"11\" font-weight=\"bold\" fill=\"{}\" text-anchor=\"middle\">{}</text>\n", FF_BOLD, badge_color, verified));
    svg.push_str(&format!("  <text x=\"24\" y=\"86\" {} font-size=\"12\" fill=\"#4a5568\" font-style=\"italic\">&quot;{}&quot;</text>\n", FF, tagline));
    svg.push_str(&text_nodes);
    svg.push_str("  <line x1=\"24\" y1=\"300\" x2=\"510\" y2=\"300\" stroke=\"#1e2d45\" stroke-width=\"1\"/>\n");
    svg.push_str(&format!("  <text x=\"24\" y=\"322\" {} font-size=\"11\" fill=\"#4a5568\">openibank.com/verify/{}</text>\n", FF, tx_id));
    svg.push_str(&format!("  <g transform=\"translate(530, 230) scale(0.85)\">{}</g>\n", qr_svg));
    svg.push_str("</svg>");
    svg
}

fn qr_code_svg(url: &str) -> String {
    use qrcode::QrCode;
    use qrcode::render::svg;
    let code = match QrCode::new(url.as_bytes()) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    code.render::<svg::Color>()
        .min_dimensions(80, 80)
        .max_dimensions(100, 100)
        .dark_color(svg::Color("#00d4ff"))
        .light_color(svg::Color("#131929"))
        .build()
}

// ── HTML Badge ────────────────────────────────────────────────────────────────

pub fn render_html(receipt: &Receipt) -> String {
    let verified = if receipt.receipt_sig.is_empty() { "UNSIGNED" } else { "✓ VERIFIED" };
    let badge_color = if receipt.receipt_sig.is_empty() { "#ffab40" } else { "#00e676" };
    let amount = receipt.iusd_amount().to_display_string();
    format!(
        r#"<div style="display:inline-block;background:#0a0e1a;border:1px solid #1e2d45;border-radius:8px;padding:12px 16px;font-family:'Courier New',monospace;font-size:12px;color:#c8d0e0;min-width:200px">
  <div style="color:{badge_color};font-weight:bold;font-size:11px;margin-bottom:6px">{verified}</div>
  <div style="color:#00d4ff;font-size:11px;margin-bottom:4px">{tx_id}</div>
  <div style="margin-bottom:2px"><span style="color:#4a5568">FROM</span> {from} → {to}</div>
  <div style="color:#00e676;font-size:14px;font-weight:bold;margin:4px 0">{amount}</div>
  <a href="{url}" style="color:#00d4ff;font-size:10px;text-decoration:none">→ Verify on-chain</a>
</div>"#,
        badge_color = badge_color,
        verified = verified,
        tx_id = escape_xml(&shorten(&receipt.tx_id, 30)),
        from = escape_xml(&receipt.from.0),
        to = escape_xml(&receipt.to.0),
        amount = escape_xml(&amount),
        url = escape_xml(&receipt.verify_url()),
    )
}

// ── File I/O ──────────────────────────────────────────────────────────────────

pub fn write_cards(receipt: &Receipt, out_dir: &Path, format: CardFormat) -> Result<Vec<PathBuf>, DomainError> {
    std::fs::create_dir_all(out_dir)?;
    let base = &receipt.tx_id;
    let mut files = Vec::new();
    if matches!(format, CardFormat::Ascii | CardFormat::Both | CardFormat::All) {
        let p = out_dir.join(format!("{}_card.txt", base));
        std::fs::write(&p, render_ascii(receipt))?;
        files.push(p);
    }
    if matches!(format, CardFormat::Svg | CardFormat::Both | CardFormat::All) {
        let p = out_dir.join(format!("{}_card.svg", base));
        std::fs::write(&p, render_svg(receipt))?;
        files.push(p);
    }
    if matches!(format, CardFormat::Html | CardFormat::All) {
        let p = out_dir.join(format!("{}_card.html", base));
        std::fs::write(&p, render_html(receipt))?;
        files.push(p);
    }
    Ok(files)
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&apos;")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;
    use crate::{AgentId, Receipt};
    use super::*;

    fn sample() -> Receipt {
        Receipt::new_unsigned(
            AgentId::new("buyer-01"), AgentId::new("seller-01"),
            50_250_000, "perm_01", "cmmt_01", "wl:abc123", "wll_evt_01",
            "deadbeef", "AI agents need banks too.",
        ).sign(&SigningKey::from_bytes(&[9u8; 32])).expect("sign")
    }

    #[test]
    fn ascii_card_contains_worldline_pointer() {
        let card = render_ascii(&sample());
        assert!(card.contains("wl:abc123#wll_evt_01"), "WLL pointer missing:\n{}", card);
    }

    #[test]
    fn ascii_card_contains_verified() {
        assert!(render_ascii(&sample()).contains("VERIFIED"));
    }

    #[test]
    fn svg_card_contains_tx_id() {
        let r = sample();
        let card = render_svg(&r);
        assert!(card.contains("ID:"), "svg missing ID field");
    }

    #[test]
    fn svg_card_contains_qr() {
        let card = render_svg(&sample());
        assert!(card.contains("<rect"), "SVG must contain QR (rect elements)");
    }

    #[test]
    fn html_card_embeddable() {
        let card = render_html(&sample());
        assert!(card.contains("<div"), "HTML must be a div");
        assert!(card.contains("VERIFIED"));
        assert!(card.contains("openibank.com/verify"));
    }

    #[test]
    fn ascii_card_nonempty() {
        assert!(!render_ascii(&sample()).is_empty());
    }
}
