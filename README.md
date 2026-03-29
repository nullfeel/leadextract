# leadextract

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)

Extract contact information (emails, phones, social links, names) from public web pages.

> **Early prototype.** Expect rough edges and breaking changes.

## Features

- Extract emails (regex + `mailto:` links)
- Extract phone numbers (international formats, `tel:` links)
- Extract social media links (GitHub, LinkedIn, Twitter/X, Instagram, Facebook, YouTube, TikTok)
- Extract names from meta tags and Schema.org structured data
- Crawl internal links up to N levels deep
- Automatic deduplication
- Export to JSON or CSV
- Concurrent crawling (up to 5 simultaneous requests)
- Rate limiting (1 req/sec per domain by default)
- Progress output to stderr

## Installation

```bash
cargo install --path .
```

## Usage

### Single page

```bash
leadextract https://example.com
```

### Crawl with depth

```bash
leadextract https://example.com --depth 2
```

### Export to JSON

```bash
leadextract https://example.com -o leads.json
```

### Export to CSV

```bash
leadextract https://example.com -o leads.csv
```

### Batch from file

```bash
# urls.txt - one URL per line
leadextract urls.txt
```

### Custom rate limit

```bash
leadextract https://example.com --rate-limit 2000  # 2 seconds between requests
```

## CSV Format

The CSV output is flattened with one row per contact item:

| source_url | type | value | platform | username |
|---|---|---|---|---|
| https://example.com | email | hello@example.com | | |
| https://example.com | phone | +1 (555) 123-4567 | | |
| https://example.com | social | https://github.com/user | github | user |

## Disclaimer

**For educational purposes only.** Always respect website terms of service, `robots.txt`, and applicable privacy laws (GDPR, CCPA, etc.) when using this tool. Do not use for spam or unsolicited contact. The authors assume no liability for misuse.

---

Built by [nullfeel](https://github.com/nullfeel)
