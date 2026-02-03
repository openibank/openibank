//! Display utilities for the CLI

use colored::*;

/// Print a section header
pub fn section(title: &str) {
    println!();
    println!("{}", "━".repeat(60).bright_black());
    println!(" {}", title.bright_white().bold());
    println!("{}", "━".repeat(60).bright_black());
}

/// Print a success message
pub fn success(message: &str) {
    println!("  {} {}", "✓".bright_green(), message);
}

/// Print an error message
pub fn error(message: &str) {
    println!("  {} {}", "✗".bright_red(), message.bright_red());
}

/// Print an info message
pub fn info(message: &str) {
    println!("  {} {}", "→".bright_blue(), message);
}

/// Print a warning message
pub fn warning(message: &str) {
    println!("  {} {}", "⚠".yellow(), message.yellow());
}

/// Print a key-value pair
pub fn kv(key: &str, value: &str) {
    println!("      {}: {}", key, value.bright_cyan());
}

/// Print a labeled value
pub fn labeled(label: &str, value: &str) {
    println!("  {}: {}", label.bright_white(), value.bright_cyan());
}
