use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::Instant;
use url::Url;

use crate::extractor;
use crate::lead::Lead;

/// Skip responses with these content types.
const SKIP_CONTENT_TYPES: &[&str] = &[
    "image/", "video/", "audio/", "application/pdf", "application/zip", "application/octet",
    "font/", "application/wasm",
];

/// Skip URLs ending with these extensions.
const SKIP_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".ico", ".bmp", ".pdf", ".zip", ".gz",
    ".tar", ".rar", ".7z", ".exe", ".dll", ".woff", ".woff2", ".ttf", ".eot", ".mp3", ".mp4",
    ".avi", ".mov", ".wmv", ".flv", ".css", ".js", ".map", ".wasm",
];

/// Per-domain rate limiter state.
struct RateLimiter {
    last_request: std::collections::HashMap<String, Instant>,
    min_interval: Duration,
}

impl RateLimiter {
    fn new(min_interval: Duration) -> Self {
        Self {
            last_request: std::collections::HashMap::new(),
            min_interval,
        }
    }

    async fn wait_for(&mut self, domain: &str) {
        if let Some(last) = self.last_request.get(domain) {
            let elapsed = last.elapsed();
            if elapsed < self.min_interval {
                tokio::time::sleep(self.min_interval - elapsed).await;
            }
        }
        self.last_request
            .insert(domain.to_string(), Instant::now());
    }
}

/// Crawl a URL up to `max_depth` levels deep, extracting leads from each page.
pub async fn crawl(start_url: &str, max_depth: u32, rate_limit_ms: u64) -> Result<Vec<Lead>, String> {
    let start_parsed = Url::parse(start_url).map_err(|e| format!("Invalid URL: {e}"))?;
    let base_domain = start_parsed
        .host_str()
        .ok_or("URL has no host")?
        .to_string();

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("leadextract/0.1 (contact info extractor)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let visited: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(Duration::from_millis(rate_limit_ms))));
    let semaphore = Arc::new(Semaphore::new(5)); // Max 5 concurrent requests
    let leads: Arc<Mutex<Vec<Lead>>> = Arc::new(Mutex::new(Vec::new()));

    // BFS queue: (url, depth)
    let queue: Arc<Mutex<VecDeque<(String, u32)>>> = Arc::new(Mutex::new(VecDeque::new()));
    queue.lock().await.push_back((start_url.to_string(), 0));
    visited.lock().await.insert(normalize_url(start_url));

    // Process the BFS queue
    loop {
        // Drain available items from the queue
        let batch: Vec<(String, u32)> = {
            let mut q = queue.lock().await;
            let mut batch = Vec::new();
            while let Some(item) = q.pop_front() {
                batch.push(item);
                if batch.len() >= 5 {
                    break;
                }
            }
            batch
        };

        if batch.is_empty() {
            // Check if there are any pending tasks by waiting briefly
            // If queue is still empty after processing, we're done
            break;
        }

        let mut handles = Vec::new();

        for (url, depth) in batch {
            let client = client.clone();
            let visited = visited.clone();
            let rate_limiter = rate_limiter.clone();
            let semaphore = semaphore.clone();
            let leads = leads.clone();
            let queue = queue.clone();
            let base_domain = base_domain.clone();
            let max_depth = max_depth;

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();

                // Rate limit
                {
                    let mut rl = rate_limiter.lock().await;
                    rl.wait_for(&base_domain).await;
                }

                // Fetch page
                let html = match fetch_page(&client, &url).await {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("  [!] Failed to fetch {url}: {e}");
                        return;
                    }
                };

                // Extract leads
                let lead = extractor::extract_all(&html, &url);
                if lead.has_data() {
                    let email_count = lead.emails.len();
                    let phone_count = lead.phones.len();
                    let social_count = lead.socials.len();
                    eprintln!(
                        "  [+] {url} => {email_count} email(s), {phone_count} phone(s), {social_count} social(s)"
                    );
                    leads.lock().await.push(lead);
                } else {
                    eprintln!("  [-] {url} => no contact info found");
                }

                // If we haven't reached max depth, extract internal links
                if depth < max_depth {
                    let links = extract_internal_links(&html, &url, &base_domain);
                    let mut vis = visited.lock().await;
                    let mut q = queue.lock().await;
                    for link in links {
                        let normalized = normalize_url(&link);
                        if !vis.contains(&normalized) {
                            vis.insert(normalized);
                            q.push_back((link, depth + 1));
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks in this batch
        for handle in handles {
            let _ = handle.await;
        }
    }

    let result = leads.lock().await.clone();
    Ok(result)
}

/// Fetch a page's HTML content, skipping binary/non-HTML responses.
async fn fetch_page(client: &Client, url: &str) -> Result<String, String> {
    // Skip known binary extensions
    let url_lower = url.to_lowercase();
    for ext in SKIP_EXTENSIONS {
        if url_lower.ends_with(ext) {
            return Err(format!("Skipped binary extension: {ext}"));
        }
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    // Check content type
    if let Some(ct) = response.headers().get("content-type") {
        let ct_str = ct.to_str().unwrap_or("");
        for skip in SKIP_CONTENT_TYPES {
            if ct_str.contains(skip) {
                return Err(format!("Skipped content type: {ct_str}"));
            }
        }
    }

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read body: {e}"))
}

/// Extract internal links (same domain) from an HTML page.
fn extract_internal_links(html: &str, page_url: &str, base_domain: &str) -> Vec<String> {
    let mut links = Vec::new();
    let document = Html::parse_document(html);

    let selector = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return links,
    };

    let base_url = match Url::parse(page_url) {
        Ok(u) => u,
        Err(_) => return links,
    };

    for element in document.select(&selector) {
        let href = match element.value().attr("href") {
            Some(h) => h.trim(),
            None => continue,
        };

        // Skip fragments, javascript, mailto, tel
        if href.starts_with('#')
            || href.starts_with("javascript:")
            || href.starts_with("mailto:")
            || href.starts_with("tel:")
            || href.starts_with("data:")
        {
            continue;
        }

        // Resolve relative URLs
        let resolved = match base_url.join(href) {
            Ok(u) => u,
            Err(_) => continue,
        };

        // Only follow internal links
        if let Some(host) = resolved.host_str() {
            if host == base_domain || host.ends_with(&format!(".{base_domain}")) {
                let mut url_str = resolved.to_string();
                // Remove fragment
                if let Some(pos) = url_str.find('#') {
                    url_str.truncate(pos);
                }
                links.push(url_str);
            }
        }
    }

    links
}

/// Normalize a URL for deduplication (strip fragment, trailing slash).
fn normalize_url(url: &str) -> String {
    let mut s = url.to_string();
    if let Some(pos) = s.find('#') {
        s.truncate(pos);
    }
    s = s.trim_end_matches('/').to_string();
    s.to_lowercase()
}
