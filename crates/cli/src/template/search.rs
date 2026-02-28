//! Implementation of the `tenor search` subcommand.
//!
//! Queries the registry API and displays matching templates with name, version,
//! description, author, category, download count, and rating.

use crate::OutputFormat;

use super::registry::RegistryClient;

/// Run the `tenor search` subcommand.
///
/// # Arguments
///
/// * `query` — search query string
/// * `category` — optional category filter
/// * `tag` — optional tag filter
/// * `registry_url` — registry endpoint override
/// * `output` — output format (text / JSON)
/// * `quiet` — suppress non-essential output
pub fn cmd_search(
    query: &str,
    category: Option<&str>,
    tag: Option<&str>,
    registry_url: Option<&str>,
    output: OutputFormat,
    quiet: bool,
) {
    let client = RegistryClient::new(registry_url, None);

    let results = match client.search(query, category, tag) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if quiet {
        return;
    }

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&results).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Text => {
            if results.is_empty() {
                println!("No templates found matching \"{}\"", query);
            } else {
                println!(
                    "Found {} template(s) matching \"{}\":",
                    results.len(),
                    query
                );
                println!();

                for result in &results {
                    // Name + version left-aligned, author right-padded to 60 chars total
                    let name_ver = format!("{} v{}", result.name, result.version);
                    let author_label = format!("by {}", result.author);
                    let padding = 60usize.saturating_sub(name_ver.len() + author_label.len());
                    println!(
                        "{}{:>width$}",
                        name_ver,
                        author_label,
                        width = author_label.len() + padding
                    );

                    // Description
                    println!("  {}", result.description);

                    // Category, downloads, rating
                    let rating_str = match result.rating {
                        Some(r) => format!("{:.1}/5", r),
                        None => "N/A".to_string(),
                    };
                    println!(
                        "  Category: {}  |  Downloads: {}  |  Rating: {}",
                        result.category, result.downloads, rating_str
                    );
                    println!();
                }
            }
        }
    }
}
