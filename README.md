<div align="center">

# leadextract

**Extract contact information from public web pages.**

Emails · Phone numbers · Social profiles · Recursive crawling · JSON/CSV export

![Rust](https://img.shields.io/badge/Rust-0d0d0d?style=flat-square&logo=rust&logoColor=e43717)
![License](https://img.shields.io/badge/License-MIT-0d0d0d?style=flat-square)

> Early prototype. May contain bugs. Contributions welcome.
> For educational and authorized use only. Respect website terms of service and privacy laws.

</div>

---

## Features

- **Email extraction** -- regex pattern matching across page content plus `mailto:` link parsing, with false-positive filtering for image filenames, bundler artifacts, and placeholder domains
- **Phone number extraction** -- international formats (`+1 (555) 123-4567`, `555-867-5309`, `555.867.5309`), `tel:` link parsing, and automatic normalization with digit-count validation (7--15 digits)
- **Social profile detection** -- identifies links to GitHub, LinkedIn, Twitter/X, Instagram, Facebook, YouTube, TikTok, and Mastodon, with automatic username extraction per platform
- **Name extraction** -- pulls names from `<meta name="author">`, `DC.creator`, `og:site_name`, and Schema.org `Person` entities in JSON-LD
- **Recursive crawling** -- BFS traversal of internal links up to N levels deep, staying within the same domain
- **Concurrent requests** -- up to 5 simultaneous fetches with a semaphore-based limiter
- **Per-domain rate limiting** -- configurable delay between requests to the same host (default: 1000ms)
- **Automatic deduplication** -- all extracted data is deduplicated before output
- **Batch mode** -- pass a text file with one URL per line instead of a single URL
- **Smart URL handling** -- auto-prepends `https://` when the scheme is missing, skips binary files and non-HTML content types
- **Export** -- save results to `.json` (structured) or `.csv` (flattened, one row per contact item)

## Install

```bash
git clone https://github.com/nullfeel/leadextract.git
cd leadextract
cargo install --path .
```

## Usage

### Extract from a single page

```bash
leadextract https://example.com
```

### Crawl a site two levels deep

```bash
# Follows internal links up to depth 2
leadextract https://example.com --depth 2
```

### Export results to JSON

```bash
leadextract https://example.com -o results.json
```

### Export results to CSV

```bash
leadextract https://example.com -o results.csv
```

### Batch extraction from a URL list

```bash
# urls.txt -- one URL per line, lines starting with # are ignored
leadextract urls.txt
```

### Custom rate limit

```bash
# Wait 2 seconds between requests to the same domain
leadextract https://example.com --rate-limit 2000
```

### Crawl with export and custom rate limit

```bash
leadextract https://example.com --depth 3 --rate-limit 500 -o leads.json
```

### Domain without scheme

```bash
# Automatically assumes https://
leadextract example.com
```

## Flags

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `target` | | _(required)_ | URL to extract from, or path to a text file containing URLs |
| `--depth` | `-d` | `0` | Maximum crawl depth (0 = single page only) |
| `--output` | `-o` | _(none)_ | Output file path; format inferred from extension (`.json` or `.csv`) |
| `--rate-limit` | | `1000` | Delay in milliseconds between requests to the same domain |

## Example Output

### Terminal

```
leadextract v0.1.0 - processing 1 URL(s), depth=1

Crawling: https://example.com
  [+] https://example.com => 2 email(s), 1 phone(s), 3 social(s)
  [+] https://example.com/about => 1 email(s), 0 phone(s), 2 social(s)
  [-] https://example.com/blog => no contact info found

Found: 3 email(s), 1 phone(s), 5 social(s), 1 name(s) across 2 page(s)

--- https://example.com ---
  Names:
    John Doe
  Emails:
    hello@example.com
    sales@example.com
  Phones:
    +1 (555) 123-4567
  Social:
    [github] https://github.com/nullfeel (@nullfeel)
    [twitter] https://twitter.com/nullfeel (@nullfeel)
    [linkedin] https://linkedin.com/in/johndoe (@johndoe)
```

### JSON export

```json
[
  {
    "url": "https://example.com",
    "emails": ["hello@example.com", "sales@example.com"],
    "phones": ["+1 (555) 123-4567"],
    "socials": [
      {
        "platform": "github",
        "url": "https://github.com/nullfeel",
        "username": "nullfeel"
      }
    ],
    "names": ["John Doe"]
  }
]
```

### CSV export

| source_url | type | value | platform | username |
|---|---|---|---|---|
| https://example.com | name | John Doe | | |
| https://example.com | email | hello@example.com | | |
| https://example.com | phone | +1 (555) 123-4567 | | |
| https://example.com | social | https://github.com/nullfeel | github | nullfeel |

## Supported Platforms

| Platform | URL patterns |
|----------|-------------|
| GitHub | `github.com/` |
| LinkedIn | `linkedin.com/in/`, `linkedin.com/company/` |
| Twitter / X | `twitter.com/`, `x.com/` |
| Instagram | `instagram.com/` |
| Facebook | `facebook.com/`, `fb.com/` |
| YouTube | `youtube.com/`, `youtu.be/` |
| TikTok | `tiktok.com/@` |
| Mastodon | `mastodon.social/@` |

## Architecture

```
src/
  main.rs        -- CLI parsing, target resolution, orchestration
  crawler.rs     -- BFS crawl engine, rate limiter, concurrent fetcher
  extractor.rs   -- Email, phone, social, and name extraction logic
  lead.rs        -- Lead and Social data structures, deduplication
  output.rs      -- Terminal display, JSON export, CSV export
```

**Crawl flow:**

1. Parse the target (single URL, bare domain, or file of URLs)
2. For each URL, start a BFS crawl with a shared visited set
3. Fetch each page with rate limiting and concurrency control (semaphore of 5)
4. Skip binary content types and known non-HTML extensions
5. Extract emails, phones, social links, and names from HTML
6. If depth allows, discover and enqueue internal links (same domain only)
7. Deduplicate all results
8. Print to terminal and optionally export to file

## How It Works

**Email extraction** uses a broad regex (`[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}`) combined with `mailto:` link parsing. False positives like image filenames (`logo@2x.png`) and known noise domains (`sentry.io`, `example.com`) are filtered out.

**Phone extraction** strips HTML tags to work on plain text, then applies three regex patterns covering international prefixed numbers, parenthesized area codes, and dash/dot-separated formats. Results are validated by digit count (7--15) and normalized. `tel:` links are also parsed.

**Social detection** scans all `<a href>` elements against a table of known platform URL patterns. Usernames are extracted from the URL path with platform-specific logic (e.g., stripping `/in/` for LinkedIn, `@` for TikTok).

**Name extraction** checks `meta[name=author]`, `meta[name=DC.creator]`, `meta[property=og:site_name]`, and `@type: Person` entities in JSON-LD blocks.

## Requirements

- Rust 1.75+
- Internet access to target URLs

## License

[MIT](LICENSE)

---

<div align="center">

Built by [nullfeel](https://github.com/nullfeel)

</div>
