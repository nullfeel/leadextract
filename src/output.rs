use std::fs::File;
use std::io::Write;
use std::path::Path;

use colored::Colorize;

use crate::lead::Lead;

/// Print results to the terminal with colors.
pub fn print_results(leads: &[Lead]) {
    if leads.is_empty() {
        println!("{}", "No contact information found.".yellow());
        return;
    }

    let total_emails: usize = leads.iter().map(|l| l.emails.len()).sum();
    let total_phones: usize = leads.iter().map(|l| l.phones.len()).sum();
    let total_socials: usize = leads.iter().map(|l| l.socials.len()).sum();
    let total_names: usize = leads.iter().map(|l| l.names.len()).sum();

    println!();
    println!(
        "{}",
        format!(
            "Found: {} email(s), {} phone(s), {} social(s), {} name(s) across {} page(s)",
            total_emails,
            total_phones,
            total_socials,
            total_names,
            leads.len()
        )
        .green()
        .bold()
    );
    println!();

    for lead in leads {
        println!("{}", format!("--- {} ---", lead.url).cyan().bold());

        if !lead.names.is_empty() {
            println!("  {}", "Names:".white().bold());
            for name in &lead.names {
                println!("    {}", name);
            }
        }

        if !lead.emails.is_empty() {
            println!("  {}", "Emails:".white().bold());
            for email in &lead.emails {
                println!("    {}", email.yellow());
            }
        }

        if !lead.phones.is_empty() {
            println!("  {}", "Phones:".white().bold());
            for phone in &lead.phones {
                println!("    {}", phone.yellow());
            }
        }

        if !lead.socials.is_empty() {
            println!("  {}", "Social:".white().bold());
            for social in &lead.socials {
                let username_str = social
                    .username
                    .as_ref()
                    .map(|u| format!(" (@{u})"))
                    .unwrap_or_default();
                println!(
                    "    [{}] {}{}",
                    social.platform.blue(),
                    social.url,
                    username_str.dimmed()
                );
            }
        }

        println!();
    }
}

/// Export leads to a JSON file.
pub fn export_json(leads: &[Lead], path: &Path) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(leads).map_err(|e| format!("JSON serialization error: {e}"))?;
    let mut file = File::create(path).map_err(|e| format!("Failed to create {}: {e}", path.display()))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    println!(
        "{}",
        format!("Exported {} lead(s) to {}", leads.len(), path.display()).green()
    );
    Ok(())
}

/// Export leads to a CSV file (flattened: one row per contact item).
pub fn export_csv(leads: &[Lead], path: &Path) -> Result<(), String> {
    let mut wtr =
        csv::Writer::from_path(path).map_err(|e| format!("Failed to create {}: {e}", path.display()))?;

    // Header
    wtr.write_record(["source_url", "type", "value", "platform", "username"])
        .map_err(|e| format!("CSV write error: {e}"))?;

    for lead in leads {
        for name in &lead.names {
            wtr.write_record([&lead.url, "name", name, "", ""])
                .map_err(|e| format!("CSV write error: {e}"))?;
        }
        for email in &lead.emails {
            wtr.write_record([&lead.url, "email", email, "", ""])
                .map_err(|e| format!("CSV write error: {e}"))?;
        }
        for phone in &lead.phones {
            wtr.write_record([&lead.url, "phone", phone, "", ""])
                .map_err(|e| format!("CSV write error: {e}"))?;
        }
        for social in &lead.socials {
            wtr.write_record([
                &lead.url,
                "social",
                &social.url,
                &social.platform,
                social.username.as_deref().unwrap_or(""),
            ])
            .map_err(|e| format!("CSV write error: {e}"))?;
        }
    }

    wtr.flush().map_err(|e| format!("CSV flush error: {e}"))?;
    println!(
        "{}",
        format!("Exported {} lead(s) to {}", leads.len(), path.display()).green()
    );
    Ok(())
}
