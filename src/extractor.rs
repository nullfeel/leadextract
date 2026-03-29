use regex::Regex;
use scraper::{Html, Selector};

use crate::lead::{Lead, Social};

/// Image/binary extensions to reject from email matches.
const FALSE_EMAIL_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".bmp", ".ico", ".tiff", ".pdf", ".zip",
    ".gz", ".tar", ".rar", ".7z", ".exe", ".dll", ".woff", ".woff2", ".ttf", ".eot", ".mp3",
    ".mp4", ".avi", ".mov", ".css", ".js", ".map",
];

/// Known social platforms and their URL patterns.
const SOCIAL_PLATFORMS: &[(&str, &[&str])] = &[
    ("github", &["github.com/"]),
    ("linkedin", &["linkedin.com/in/", "linkedin.com/company/"]),
    ("twitter", &["twitter.com/", "x.com/"]),
    ("instagram", &["instagram.com/"]),
    ("facebook", &["facebook.com/", "fb.com/"]),
    ("youtube", &["youtube.com/", "youtu.be/"]),
    ("tiktok", &["tiktok.com/@"]),
    ("mastodon", &["mastodon.social/@"]),
];

/// Extract all lead information from an HTML page.
pub fn extract_all(html: &str, source_url: &str) -> Lead {
    let mut lead = Lead::new(source_url.to_string());

    lead.emails = extract_emails(html);
    lead.phones = extract_phones(html);
    lead.socials = extract_socials(html);
    lead.names = extract_names(html);
    lead.dedup();

    lead
}

/// Extract email addresses from HTML content via regex and mailto: links.
pub fn extract_emails(html: &str) -> Vec<String> {
    let mut emails = Vec::new();

    // Regex-based extraction
    let re = Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap();
    for cap in re.find_iter(html) {
        let email = cap.as_str().to_lowercase();
        if !is_false_email(&email) {
            emails.push(email);
        }
    }

    // mailto: link extraction
    let document = Html::parse_document(html);
    if let Ok(selector) = Selector::parse("a[href^='mailto:']") {
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                let addr = href
                    .trim_start_matches("mailto:")
                    .split('?')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_lowercase();
                if !addr.is_empty() && !is_false_email(&addr) {
                    emails.push(addr);
                }
            }
        }
    }

    emails
}

/// Check if an email-like match is actually a filename or other false positive.
fn is_false_email(email: &str) -> bool {
    FALSE_EMAIL_EXTENSIONS
        .iter()
        .any(|ext| email.ends_with(ext))
        || email.contains("example.com")
        || email.contains("sentry.io")
        || email.contains("webpack")
        || email.starts_with("data:")
}

/// Extract phone numbers in various international formats.
pub fn extract_phones(html: &str) -> Vec<String> {
    let mut phones = Vec::new();

    // Strip HTML tags for cleaner text matching
    let text = strip_tags(html);

    let patterns = [
        // +1 (234) 567-8901 or +44 20 7946 0958
        r"\+\d{1,3}[\s\-]?\(?\d{1,4}\)?[\s\-]?\d{2,4}[\s\-]?\d{2,4}[\s\-]?\d{0,4}",
        // (234) 567-8901
        r"\(\d{3}\)\s?\d{3}[\-\s]\d{4}",
        // 234-567-8901 or 234.567.8901
        r"\b\d{3}[\-\.]\d{3}[\-\.]\d{4}\b",
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        for cap in re.find_iter(&text) {
            let phone = cap.as_str().trim().to_string();
            // Filter out numbers that are too short or too long
            let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 7 && digits.len() <= 15 {
                phones.push(normalize_phone(&phone));
            }
        }
    }

    // Also extract from tel: links
    let document = Html::parse_document(html);
    if let Ok(selector) = Selector::parse("a[href^='tel:']") {
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                let num = href.trim_start_matches("tel:").trim().to_string();
                if !num.is_empty() {
                    phones.push(normalize_phone(&num));
                }
            }
        }
    }

    phones
}

/// Normalize whitespace in phone numbers.
fn normalize_phone(phone: &str) -> String {
    phone.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

/// Extract social media links and attempt to parse usernames.
pub fn extract_socials(html: &str) -> Vec<Social> {
    let mut socials = Vec::new();
    let document = Html::parse_document(html);

    let selector = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return socials,
    };

    for element in document.select(&selector) {
        let href = match element.value().attr("href") {
            Some(h) => h.trim().to_string(),
            None => continue,
        };

        for &(platform, patterns) in SOCIAL_PLATFORMS {
            let href_lower = href.to_lowercase();
            for &pattern in patterns {
                if href_lower.contains(pattern) {
                    let username = extract_username(&href, platform);
                    socials.push(Social {
                        platform: platform.to_string(),
                        url: href.clone(),
                        username,
                    });
                    break;
                }
            }
        }
    }

    socials
}

/// Try to extract a username from a social media URL.
fn extract_username(url: &str, platform: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let path = parsed.path().trim_matches('/');

    if path.is_empty() {
        return None;
    }

    let username = match platform {
        "linkedin" => {
            // linkedin.com/in/username or linkedin.com/company/name
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 2 {
                Some(parts[1].to_string())
            } else {
                None
            }
        }
        "tiktok" | "mastodon" => {
            // Handle @username in path
            let segment = path.split('/').next()?;
            Some(segment.trim_start_matches('@').to_string())
        }
        _ => {
            // github.com/user, twitter.com/user, etc.
            let segment = path.split('/').next()?;
            let username = segment.trim_start_matches('@');
            // Ignore generic paths
            if matches!(
                username.to_lowercase().as_str(),
                "share" | "intent" | "hashtag" | "search" | "explore" | "settings" | "about"
            ) {
                None
            } else {
                Some(username.to_string())
            }
        }
    };

    username.filter(|u| !u.is_empty())
}

/// Extract names from meta tags and structured data.
pub fn extract_names(html: &str) -> Vec<String> {
    let mut names = Vec::new();
    let document = Html::parse_document(html);

    // Meta author tag
    if let Ok(sel) = Selector::parse("meta[name='author']") {
        for el in document.select(&sel) {
            if let Some(content) = el.value().attr("content") {
                let name = content.trim().to_string();
                if !name.is_empty() && name.len() < 100 {
                    names.push(name);
                }
            }
        }
    }

    // DC.creator
    if let Ok(sel) = Selector::parse("meta[name='DC.creator']") {
        for el in document.select(&sel) {
            if let Some(content) = el.value().attr("content") {
                let name = content.trim().to_string();
                if !name.is_empty() && name.len() < 100 {
                    names.push(name);
                }
            }
        }
    }

    // og:site_name (sometimes contains company/person names)
    if let Ok(sel) = Selector::parse("meta[property='og:site_name']") {
        for el in document.select(&sel) {
            if let Some(content) = el.value().attr("content") {
                let name = content.trim().to_string();
                if !name.is_empty() && name.len() < 100 {
                    names.push(name);
                }
            }
        }
    }

    // Schema.org Person name from JSON-LD
    let re = Regex::new(r#""@type"\s*:\s*"Person"[^}]*"name"\s*:\s*"([^"]+)""#).unwrap();
    for cap in re.captures_iter(html) {
        if let Some(name) = cap.get(1) {
            let n = name.as_str().trim().to_string();
            if !n.is_empty() && n.len() < 100 {
                names.push(n);
            }
        }
    }

    // Also try reverse order in JSON-LD
    let re2 = Regex::new(r#""name"\s*:\s*"([^"]+)"[^}]*"@type"\s*:\s*"Person""#).unwrap();
    for cap in re2.captures_iter(html) {
        if let Some(name) = cap.get(1) {
            let n = name.as_str().trim().to_string();
            if !n.is_empty() && n.len() < 100 {
                names.push(n);
            }
        }
    }

    names
}

/// Strip HTML tags to get plain text (simple approach).
fn strip_tags(html: &str) -> String {
    let re = Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(html, " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_emails() {
        let html = r#"
            <p>Contact us at hello@company.com or sales@company.com</p>
            <a href="mailto:info@company.com?subject=Hi">Email us</a>
            <img src="logo@2x.png" />
        "#;
        let emails = extract_emails(html);
        assert!(emails.contains(&"hello@company.com".to_string()));
        assert!(emails.contains(&"sales@company.com".to_string()));
        assert!(emails.contains(&"info@company.com".to_string()));
        // Should not contain image false positive
        assert!(!emails.iter().any(|e| e.contains(".png")));
    }

    #[test]
    fn test_extract_phones() {
        let html = r#"
            <p>Call us: +1 (555) 123-4567</p>
            <p>Or: 555-867-5309</p>
            <a href="tel:+44-20-7946-0958">UK Office</a>
        "#;
        let phones = extract_phones(html);
        assert!(!phones.is_empty());
    }

    #[test]
    fn test_extract_socials() {
        let html = r#"
            <a href="https://github.com/nullfeel">GitHub</a>
            <a href="https://twitter.com/nullfeel">Twitter</a>
            <a href="https://linkedin.com/in/johndoe">LinkedIn</a>
        "#;
        let socials = extract_socials(html);
        assert_eq!(socials.len(), 3);
        assert!(socials.iter().any(|s| s.platform == "github"));
        assert!(socials.iter().any(|s| s.platform == "twitter"));
        assert!(socials.iter().any(|s| s.platform == "linkedin"));
    }

    #[test]
    fn test_extract_names() {
        let html = r#"
            <html><head>
                <meta name="author" content="John Doe" />
                <meta property="og:site_name" content="Doe Consulting" />
            </head><body></body></html>
        "#;
        let names = extract_names(html);
        assert!(names.contains(&"John Doe".to_string()));
        assert!(names.contains(&"Doe Consulting".to_string()));
    }
}
