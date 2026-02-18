use std::path::{Path, PathBuf};

use crate::{DomainError, Receipt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardFormat {
    Ascii,
    Svg,
    Both,
}

pub fn render_ascii(receipt: &Receipt) -> String {
    format!(
        "╔══════════════════════════════════════════════════════════════════╗\n\
         ║ OpenIBank Receipt Card                                      ║\n\
         ╠══════════════════════════════════════════════════════════════════╣\n\
         ║ tx_id: {tx_id}\n\
         ║ from→to: {from} -> {to}\n\
         ║ amount: {amount}\n\
         ║ permit_id: {permit}\n\
         ║ commitment_id: {commitment}\n\
         ║ wll: {wll}\n\
         ║ event_hash: {event_hash}\n\
         ║ sig(ed25519): {sig}\n\
         ║ time: {time}\n\
         ║ {tagline}\n\
         ╚══════════════════════════════════════════════════════════════════╝\n",
        tx_id = receipt.tx_id,
        from = receipt.from,
        to = receipt.to,
        amount = receipt.amount,
        permit = receipt.permit_id,
        commitment = receipt.commitment_id,
        wll = receipt.worldline_pointer(),
        event_hash = receipt.worldline_event_hash,
        sig = shorten(&receipt.receipt_sig, 48),
        time = receipt.timestamp.to_rfc3339(),
        tagline = receipt.tagline
    )
}

pub fn render_svg(receipt: &Receipt) -> String {
    let lines = vec![
        "OpenIBank Receipt Card".to_string(),
        format!("tx_id: {}", receipt.tx_id),
        format!("from->to: {} -> {}", receipt.from, receipt.to),
        format!("amount: {}", receipt.amount),
        format!("permit_id: {}", receipt.permit_id),
        format!("commitment_id: {}", receipt.commitment_id),
        format!("wll: {}", receipt.worldline_pointer()),
        format!("event_hash: {}", receipt.worldline_event_hash),
        format!("sig(ed25519): {}", shorten(&receipt.receipt_sig, 52)),
        format!("time: {}", receipt.timestamp.to_rfc3339()),
        receipt.tagline.clone(),
    ];

    let mut text_nodes = String::new();
    for (idx, line) in lines.iter().enumerate() {
        let y = 45 + idx as i32 * 26;
        text_nodes.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"Menlo, monospace\" font-size=\"16\" fill=\"#0d1b2a\">{}</text>\n",
            y,
            escape_xml(line)
        ));
    }

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1100\" height=\"380\" viewBox=\"0 0 1100 380\">
  <defs>
    <linearGradient id=\"bg\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">
      <stop offset=\"0%\" stop-color=\"#e3f2fd\" />
      <stop offset=\"100%\" stop-color=\"#fef6e4\" />
    </linearGradient>
  </defs>
  <rect x=\"0\" y=\"0\" width=\"1100\" height=\"380\" rx=\"18\" fill=\"url(#bg)\" />
  <rect x=\"12\" y=\"12\" width=\"1076\" height=\"356\" rx=\"14\" fill=\"#ffffff\" stroke=\"#90caf9\" stroke-width=\"2\" />
  {}
</svg>
",
        text_nodes
    )
}

pub fn write_cards(
    receipt: &Receipt,
    out_dir: &Path,
    format: CardFormat,
) -> Result<Vec<PathBuf>, DomainError> {
    std::fs::create_dir_all(out_dir)?;
    let mut files = Vec::new();

    if matches!(format, CardFormat::Ascii | CardFormat::Both) {
        let txt = out_dir.join("receipt_card.txt");
        std::fs::write(&txt, render_ascii(receipt))?;
        files.push(txt);
    }

    if matches!(format, CardFormat::Svg | CardFormat::Both) {
        let svg = out_dir.join("receipt_card.svg");
        std::fs::write(&svg, render_svg(receipt))?;
        files.push(svg);
    }

    Ok(files)
}

fn shorten(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;

    use crate::{AgentId, Receipt};

    use super::*;

    fn sample_receipt() -> Receipt {
        let key = SigningKey::from_bytes(&[9u8; 32]);
        Receipt::new_unsigned(
            AgentId::new("buyer-01"),
            AgentId::new("seller-01"),
            2_000,
            "permit-01",
            "commitment-01",
            "wl-demo",
            "evt-demo",
            "abcd1234",
            "AI agents need banks too.",
        )
        .sign(&key)
        .expect("sign")
    }

    #[test]
    fn ascii_card_contains_worldline_pointer() {
        let receipt = sample_receipt();
        let card = render_ascii(&receipt);
        assert!(
            card.contains("wll: wl-demo#evt-demo"),
            "card should include worldline pointer"
        );
    }

    #[test]
    fn svg_card_contains_tx_id() {
        let receipt = sample_receipt();
        let card = render_svg(&receipt);
        assert!(card.contains("tx_id:"), "svg should contain tx_id");
    }
}
