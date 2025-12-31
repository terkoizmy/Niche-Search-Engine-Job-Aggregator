# 🔍 Niche Search Engine: Job Aggregator
## Presentation Slides

---

# Slide 1: Project Idea

## What We Built
A **specialized search engine** for remote programming jobs.

```
┌─────────────────────────────────────────────────────────┐
│                                                         │
│   User Query: "rust backend developer"                  │
│                      │                                  │
│                      ▼                                  │
│   ┌─────────────────────────────────┐                   │
│   │    NICHE SEARCH ENGINE          │                   │
│   │    (Job Aggregator)             │                   │
│   └─────────────────────────────────┘                   │
│                      │                                  │
│                      ▼                                  │
│   Results: Relevant remote job listings                 │
│   - Senior Rust Backend Engineer @ TechCorp             │
│   - Rust Developer @ StartupXYZ                         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Key Features
| Feature | Description |
|---------|-------------|
| 🕷️ Web Scraping | Automated data collection from WeWorkRemotely |
| 📊 Full-Text Indexing | Tantivy inverted index for fast search |
| 🌐 REST API | JSON responses via Axum web framework |
| 💰 Salary Extraction | Regex-based salary parsing |

---

# Slide 2: Background

## The Problem
- General search engines (Google, Bing) return **too broad** results
- Job seekers waste time filtering irrelevant listings
- No structured search by salary, location, or tech stack

## The Solution: Niche Search Engine
A **domain-specific** search engine that:
1. Focuses on ONE data source (WeWorkRemotely)
2. Indexes job-specific fields (title, company, salary)
3. Returns highly relevant results

## Why Rust?
| Advantage | Benefit |
|-----------|---------|
| 🚀 Performance | Memory-safe, zero-cost abstractions |
| 🔒 Reliability | No null pointer exceptions |
| 📦 Ecosystem | Tantivy (search), Axum (web), Serde (JSON) |

---

# Slide 3: Dataset Used

## Data Source
**WeWorkRemotely.com** - One of the largest remote job boards

## Scraped Categories
```
1. /remote-software-developer-jobs
2. /categories/remote-full-stack-programming-jobs
3. /categories/remote-back-end-programming-jobs
4. /categories/remote-front-end-programming-jobs
```

## Data Schema

| Field | Type | Example |
|-------|------|---------|
| `title` | String | "Senior Rust Developer" |
| `company` | String | "TechCorp Inc" |
| `location` | String | "USA / Remote" |
| `description` | String | "Full-Time Anywhere..." |
| `salary_raw` | String | "$100,000 or more USD" |
| `salary_min` | i64 | 100000 |
| `url` | String | "https://weworkremotely.com/..." |

## Sample Dataset
```json
{
  "title": "Senior Backend Engineer",
  "company": "Proton.ai",
  "location": "Cambridge, MA",
  "salary_raw": "$50,000 - $74,999 USD",
  "salary_min": 50000,
  "url": "https://weworkremotely.com/remote-jobs/..."
}
```

---

# Slide 4: Text Preprocessing

## Pipeline Overview
```
Raw HTML → Parse → Extract → Clean → Tokenize → Index
```

## Step 1: HTML Parsing
Using CSS Selectors to extract structured data:
```rust
let job_selector = Selector::parse("li.feature, .new-listing-container");
let title_selector = Selector::parse(".new-listing__header__title");
let company_selector = Selector::parse(".new-listing__company-name");
```

## Step 2: Text Cleaning
```
Input:  "  Senior Rust\n  Developer  "
Output: "Senior Rust Developer"
```
- Remove extra whitespace
- Normalize newlines
- Trim leading/trailing spaces

## Step 3: Salary Extraction (Regex)
```rust
// Pattern: \$?(\d{1,3}(?:,\d{3})*|\d+)
// Examples:
"$50,000 - $70,000"  → 50000
"100000 USD"         → 100000
"Competitive"        → None
```

## Step 4: Tokenization (Tantivy)
```
"Senior Rust Developer" → ["senior", "rust", "developer"]
```
- Lowercase conversion
- Punctuation removal
- Stop word filtering

---

# Slide 5: Modeling (Search Engine Architecture)

## Inverted Index Model

**Forward Index (Traditional):**
```
Doc1 → [senior, rust, developer]
Doc2 → [python, backend, engineer]
```

**Inverted Index (Our Model):**
```
"rust"      → [Doc1, Doc3, Doc7]
"developer" → [Doc1, Doc5, Doc8]
"python"    → [Doc2, Doc4]
```

## Tantivy Schema Definition
```rust
schema_builder.add_text_field("title", TEXT | STORED);
schema_builder.add_text_field("company", TEXT | STORED);
schema_builder.add_text_field("description", TEXT);  // Not stored
schema_builder.add_i64_field("salary_min", INDEXED);
```

## Ranking Model: BM25

```
BM25 Score = Σ IDF(term) × TF(term, doc) × (k₁ + 1)
             ─────────────────────────────────────────
             TF(term, doc) + k₁ × (1 - b + b × |doc|/avgdl)
```

| Component | Meaning |
|-----------|---------|
| **IDF** | Inverse Document Frequency (rare words = higher score) |
| **TF** | Term Frequency (more occurrences = higher score) |
| **k₁** | Saturation parameter (default: 1.2) |
| **b** | Length normalization (default: 0.75) |

## Search Query Flow
```
Query: "rust developer"
        │
        ▼
┌─────────────────────┐
│  1. Parse Query     │ → [Term("rust"), Term("developer")]
└─────────────────────┘
        │
        ▼
┌─────────────────────┐
│  2. Index Lookup    │ → rust: [doc0, doc5], developer: [doc0, doc8]
└─────────────────────┘
        │
        ▼
┌─────────────────────┐
│  3. BM25 Scoring    │ → doc0: 15.3, doc5: 8.2, doc8: 4.1
└─────────────────────┘
        │
        ▼
┌─────────────────────┐
│  4. Return Top 10   │ → JSON response
└─────────────────────┘
```

---

# Slide 6: Evaluation

## System Architecture Evaluation

| Component | Technology | Status |
|-----------|------------|--------|
| Data Collection | reqwest + scraper | ✅ Working |
| Text Processing | regex + tantivy tokenizer | ✅ Working |
| Indexing | Tantivy 0.19 | ✅ Working |
| Web API | Axum 0.6 | ✅ Working |
| Build | Cargo Workspace | ✅ No errors |

## Performance Metrics

| Metric | Value |
|--------|-------|
| Scraping Speed | ~4 pages in 2-3 seconds |
| Index Build Time | < 1 second (for ~50 jobs) |
| Query Response | < 10ms |
| Memory Usage | ~ 50MB (50MB writer heap) |

## Search Quality

**Query: "rust backend"**
```json
{
  "query": "rust backend",
  "total_results": 3,
  "results": [
    {"title": "Senior Rust Backend Engineer", "score": 12.5},
    {"title": "Backend Developer (Rust/Go)", "score": 8.3},
    {"title": "Systems Programmer", "score": 2.1}
  ]
}
```

## Limitations
| Issue | Reason |
|-------|--------|
| Website Changes | HTML selectors may break if WWR updates layout |
| Rate Limiting | Too many requests may get blocked |
| No Pagination | Only scrapes first page of each category |
| English Only | No multilingual support |

---

# Slide 7: Conclusion

## What We Achieved

✅ Built a complete **Niche Search Engine** in Rust
✅ Implemented **web scraping** with deduplication
✅ Created **inverted index** using Tantivy
✅ Deployed **REST API** with Axum
✅ Applied **BM25 ranking** for relevance scoring

## Key Learnings

| Topic | Learning |
|-------|----------|
| Information Retrieval | Inverted index, tokenization, BM25 |
| Rust Development | Workspace structure, error handling, async |
| Web Scraping | CSS selectors, regex extraction |
| API Design | REST endpoints, JSON serialization |

## Future Improvements

1. 📄 **Pagination** - Scrape all pages, not just first page
2. 🔍 **Advanced Queries** - Salary range filters, location filters
3. 🌐 **More Sources** - Add RemoteOK, Indeed, LinkedIn
4. 📱 **Frontend** - Build React/Vue UI for the search API
5. ⏰ **Scheduling** - Auto-refresh data with cron jobs
6. 🧪 **Testing** - Add integration tests for scraper

## Architecture Summary

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   SCRAPER   │────▶│   INDEXER   │────▶│   SERVER    │
│   (Rust)    │     │  (Tantivy)  │     │   (Axum)    │
└─────────────┘     └─────────────┘     └─────────────┘
      │                   │                   │
      ▼                   ▼                   ▼
 jobs.json          search_index/      localhost:3000
```

---

# Thank You! 🎉

## Project Links
- **Repository:** [GitHub Link]
- **Demo:** `cargo run -p server` → http://127.0.0.1:3000

## Quick Start
```bash
# 1. Scrape jobs
cargo run -p scraper

# 2. Start server
cargo run -p server

# 3. Search
curl "http://127.0.0.1:3000/search?q=rust"
```
