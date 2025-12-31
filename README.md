# 🔍 Niche Search Engine - Job Aggregator

A high-performance job search engine built in Rust using a workspace architecture. Scrapes remote job listings from WeWorkRemotely.com, indexes them with Tantivy full-text search, and serves results via a REST API.

---

## 🎯 What is a Niche Search Engine?

A **Niche Search Engine** is a specialized search engine focused on a specific domain or topic, unlike general-purpose search engines (Google, Bing) that index the entire web.

### Comparison: General vs Niche Search Engine

| Aspect | General Search Engine | Niche Search Engine (This Project) |
|--------|----------------------|-----------------------------------|
| **Scope** | Entire internet (billions of pages) | Single domain: remote jobs |
| **Data Source** | Web crawlers indexing everything | Targeted scraping of WeWorkRemotely |
| **Index Size** | Petabytes of data | Megabytes (hundreds/thousands of jobs) |
| **Relevance** | Broad ranking algorithms | Domain-specific (title, company, salary) |
| **Infrastructure** | Massive distributed systems | Single server, local index |
| **Use Case** | Find anything | Find remote programming jobs |

### Why Build a Niche Search Engine?

1. **Focused Results** - No noise from unrelated content
2. **Custom Schema** - Index fields relevant to your domain (salary, location)
3. **Fast & Lightweight** - Small index = millisecond queries
4. **Control** - You decide what gets indexed and how
5. **Learning** - Understand search engine internals without massive scale

### How This Project Works as a Niche Search Engine

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        NICHE SEARCH ENGINE PIPELINE                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   [WeWorkRemotely.com]                                                  │
│          │                                                              │
│          ▼                                                              │
│   ┌─────────────┐    Instead of crawling the entire web,               │
│   │  SCRAPER    │    we only fetch job listings from ONE source.       │
│   └─────────────┘                                                       │
│          │                                                              │
│          ▼                                                              │
│   ┌─────────────┐    Instead of generic text, we extract               │
│   │  INDEXER    │    structured data: title, company, salary.          │
│   └─────────────┘                                                       │
│          │                                                              │
│          ▼                                                              │
│   ┌─────────────┐    Instead of PageRank, we use                       │
│   │  SEARCHER   │    BM25 relevance on job-specific fields.            │
│   └─────────────┘                                                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 🧠 How the Indexer Works (Deep Dive)

The indexer is the **heart of any search engine**. It transforms raw data into a structure optimized for fast searching.

### What is an Inverted Index?

An **Inverted Index** is the core data structure used by search engines. Instead of storing "document → words", it stores "word → documents".

**Example: Traditional Storage (Forward Index)**
```
Job 1: "Senior Rust Developer at TechCorp"
Job 2: "Python Backend Engineer"
Job 3: "Rust Systems Programmer"
```

**Inverted Index (What Tantivy Builds)**
```
"rust"     → [Job 1, Job 3]
"senior"   → [Job 1]
"python"   → [Job 2]
"backend"  → [Job 2]
"techcorp" → [Job 1]
```

When you search for "rust", the engine instantly finds Jobs 1 and 3 without scanning every document!

### Tantivy Indexing Process (Step-by-Step)

```
┌──────────────────────────────────────────────────────────────────┐
│                    TANTIVY INDEXING PIPELINE                     │
└──────────────────────────────────────────────────────────────────┘

Step 1: SCHEMA DEFINITION
─────────────────────────
Define what fields exist and their types:

┌─────────────┬──────────┬─────────────────────────────────────────┐
│ Field       │ Type     │ Options                                 │
├─────────────┼──────────┼─────────────────────────────────────────┤
│ title       │ TEXT     │ STORED (returned in results)            │
│ company     │ TEXT     │ STORED (returned in results)            │
│ description │ TEXT     │ NOT STORED (searchable only, saves RAM) │
│ salary_min  │ I64      │ INDEXED (for numeric range queries)     │
└─────────────┴──────────┴─────────────────────────────────────────┘

Step 2: TOKENIZATION
────────────────────
Break text into searchable tokens:

"Senior Rust Developer" → ["senior", "rust", "developer"]
                              │
                    (lowercase, remove punctuation)


Step 3: BUILD INVERTED INDEX
────────────────────────────
For each token, record which documents contain it:

Token "rust":
  ├── Document ID: 0  (position: 2, field: title)
  ├── Document ID: 5  (position: 1, field: title)
  └── Document ID: 12 (position: 4, field: description)

Token "developer":
  ├── Document ID: 0  (position: 3, field: title)
  └── Document ID: 8  (position: 1, field: title)


Step 4: COMMIT TO DISK
──────────────────────
Write index segments to ./search_index/:

search_index/
├── meta.json           ← Index metadata (schema, segments)
├── .managed.json       ← Tantivy internal tracking
└── <segment_id>/       ← Actual index data
    ├── .fast           ← Fast fields (numeric data)
    ├── .idx            ← Inverted index
    ├── .pos            ← Token positions
    ├── .store          ← Stored field values
    └── .term           ← Term dictionary
```

### How Search Queries Work

When you query `/search?q=rust developer`:

```
┌─────────────────────────────────────────────────────────────────┐
│                       SEARCH FLOW                               │
└─────────────────────────────────────────────────────────────────┘

1. PARSE QUERY
   "rust developer" → [Term("rust"), Term("developer")]

2. LOOKUP INVERTED INDEX
   rust      → [doc0, doc5, doc12]
   developer → [doc0, doc8]

3. COMBINE RESULTS (intersection/union based on query type)
   Matching docs: [doc0, doc5, doc8, doc12]

4. SCORE WITH BM25
   ┌─────────┬─────────────────────────────────────────────────┐
   │ Doc ID  │ BM25 Score                                      │
   ├─────────┼─────────────────────────────────────────────────┤
   │ doc0    │ 15.32 (contains BOTH terms in title)            │
   │ doc5    │ 8.21  (contains "rust" only)                    │
   │ doc12   │ 5.67  (contains "rust" in description)          │
   │ doc8    │ 4.12  (contains "developer" only)               │
   └─────────┴─────────────────────────────────────────────────┘

5. RETURN TOP K RESULTS (sorted by score)
   → doc0, doc5, doc12, doc8 (top 10)
```

### BM25 Scoring Algorithm

Tantivy uses **BM25** (Best Match 25), the industry-standard ranking algorithm:

```
BM25(doc, query) = Σ IDF(term) × TF(term, doc) × (k₁ + 1)
                   ─────────────────────────────────────────
                   TF(term, doc) + k₁ × (1 - b + b × |doc|/avgdl)

Where:
- IDF = Inverse Document Frequency (rare words score higher)
- TF  = Term Frequency (more occurrences = higher score)
- k₁  = Term saturation parameter (default: 1.2)
- b   = Length normalization (default: 0.75)
```

**In simple terms:** Documents score higher when they:
1. Contain rare/unique query terms (IDF)
2. Contain query terms multiple times (TF)
3. Are shorter (normalized by document length)

---

## 📐 Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Rust Workspace                              │
├─────────────┬─────────────────────┬─────────────────────────────┤
│   common/   │      scraper/       │          server/            │
│  (Library)  │     (Binary)        │         (Binary)            │
├─────────────┼─────────────────────┼─────────────────────────────┤
│ Job struct  │ HTTP Client         │ Tantivy Indexer             │
│ Serde       │ HTML Parser         │ Axum Web Server             │
│             │ Regex Extractor     │ Search API                  │
└─────────────┴─────────────────────┴─────────────────────────────┘
```

## 🔄 Workflow Diagram

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   1. SCRAPER     │────▶│   2. INDEXER     │────▶│   3. SERVER      │
│                  │     │                  │     │                  │
│ Fetch HTML from  │     │ Read jobs.json   │     │ Accept HTTP      │
│ WeWorkRemotely   │     │ Build Tantivy    │     │ search queries   │
│ Parse listings   │     │ schema & index   │     │ Return JSON      │
│ Extract salary   │     │ Commit to disk   │     │ results          │
│ Save to JSON     │     │                  │     │                  │
└──────────────────┘     └──────────────────┘     └──────────────────┘
        │                        │                        │
        ▼                        ▼                        ▼
   data/jobs.json          search_index/           http://127.0.0.1:3000
```

---

## 📦 Module Details

### 1. Common Library (`common/`)

Shared data structures used by both scraper and server.

**File:** `common/src/lib.rs`

```rust
pub struct Job {
    pub title: String,       // Job title
    pub company: String,     // Company name
    pub location: String,    // Location/Region
    pub description: String, // Job description text
    pub salary_raw: String,  // Original salary text from website
    pub salary_min: Option<i64>, // Extracted minimum salary (if found)
    pub url: String,         // Direct link to job posting
}
```

**Dependencies:**
- `serde` - Serialization/deserialization for JSON

---

### 2. Scraper Binary (`scraper/`)

Web scraper that collects job listings from WeWorkRemotely.

**File:** `scraper/src/main.rs`

**Features:**
| Feature | Description |
|---------|-------------|
| Multi-URL Scraping | Scrapes 4 job category pages in sequence |
| Deduplication | Uses HashSet to prevent duplicate jobs |
| Salary Extraction | Regex-based extraction of salary numbers |
| Error Resilience | Continues to next URL if one fails |

**Target URLs:**
1. `/remote-software-developer-jobs`
2. `/categories/remote-full-stack-programming-jobs`
3. `/categories/remote-back-end-programming-jobs`
4. `/categories/remote-front-end-programming-jobs`

**CSS Selectors Used:**
```
Job Container:  li.feature, .new-listing-container
Title:          .new-listing__header__title
Company:        .new-listing__company-name
Location:       .new-listing__company-headquarters
Link:           .listing-link--unlocked, ._blank
```

**Salary Extraction Logic:**
```rust
fn extract_salary(salary_raw: &str) -> Option<i64>
// Regex: \$?(\d{1,3}(?:,\d{3})*|\d+)
// Matches: "$50,000", "100000", "$75,000 - $99,999"
// Returns first number >= 1000 (filters out noise like "21d")
```

**Output:** `data/jobs.json`

**Dependencies:**
- `reqwest` (blocking) - HTTP client
- `scraper` - HTML parsing
- `regex` - Salary extraction
- `serde_json` - JSON serialization

---

### 3. Server Binary (`server/`)

Search engine server combining Tantivy indexing with Axum REST API.

**File:** `server/src/main.rs`

#### Tantivy Indexer

**Schema Definition:**
| Field | Type | Options | Purpose |
|-------|------|---------|---------|
| `title` | TEXT | STORED | Searchable, returned in results |
| `company` | TEXT | STORED | Searchable, returned in results |
| `description` | TEXT | (not stored) | Searchable only, saves disk space |
| `salary_min` | I64 | INDEXED | For range filtering (future use) |

**Index Location:** `./search_index/`

**Indexing Process:**
1. Read `data/jobs.json` on startup
2. Create/open Tantivy index directory
3. Clear existing documents (fresh re-index)
4. Add all jobs to index with 50MB writer heap
5. Commit changes to disk

#### Axum Web Server

**Endpoints:**
| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | API info and usage help |
| GET | `/search?q=<keywords>` | Full-text job search |

**Search Response Format:**
```json
{
  "query": "rust developer",
  "total_results": 5,
  "results": [
    {
      "title": "Senior Rust Developer",
      "company": "TechCorp",
      "score": 12.345
    }
  ]
}
```

**Query Parser Configuration:**
- Searches across: `title` + `description` fields
- Returns: Top 10 results by relevance score
- Shared state via `Arc<AppState>` containing IndexReader

**Dependencies:**
- `tantivy` 0.19 - Full-text search engine
- `axum` 0.6 - Async web framework
- `tokio` - Async runtime
- `serde_json` - JSON responses

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ with Cargo
- Internet connection (for scraping)

### Step 1: Clone & Build
```bash
git clone <repository-url>
cd Niche-Search-Engine-Job-Aggregator
cargo build --workspace
```

### Step 2: Scrape Jobs
```bash
cargo run -p scraper@0.1.0
```
Output:
```
🔍 Starting WeWorkRemotely Job Scraper...
📡 Fetching jobs from: https://weworkremotely.com/...
✅ Fetched 77130 bytes
📋 Found: Senior Backend Engineer at TechCorp
...
📊 Total unique jobs found: 45
💾 Saved 45 jobs to "data/jobs.json"
```

### Step 3: Start Search Server
```bash
cargo run -p server@0.1.0
```
Output:
```
🚀 Starting Job Search Engine Server...
📂 Loading jobs from "data/jobs.json"
📊 Loaded 45 jobs
📝 Creating new index...
✅ Indexing complete!
🌐 Server running at http://127.0.0.1:3000
```

### Step 4: Search Jobs
```bash
# Search for Rust jobs
curl "http://127.0.0.1:3000/search?q=rust"

# Search for Python backend
curl "http://127.0.0.1:3000/search?q=python+backend"
```

---

## 📁 Project Structure

```
Niche-Search-Engine-Job-Aggregator/
├── Cargo.toml              # Workspace manifest
├── Cargo.lock              # Dependency lock file
├── README.md               # This file
│
├── common/                 # Shared library
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs          # Job struct definition
│
├── scraper/                # Web scraper binary
│   ├── Cargo.toml
│   └── src/
│       └── main.rs         # Scraping logic
│
├── server/                 # Search API binary
│   ├── Cargo.toml
│   └── src/
│       └── main.rs         # Indexer + Axum server
│
├── data/                   # Generated data (gitignored)
│   └── jobs.json           # Scraped job listings
│
└── search_index/           # Tantivy index (gitignored)
    ├── meta.json
    └── *.managed.json
```

---

## ⚙️ Configuration

### Dependency Constraints
These versions are specifically chosen to avoid Windows/C++ compilation issues:

```toml
# Tantivy with minimal features
tantivy = { version = "0.19", default-features = false, features = ["mmap", "stopwords"] }

# Axum compatible with Tantivy 0.19
axum = "0.6"

# Blocking HTTP for simplicity
reqwest = { version = "0.11", features = ["blocking", "json"] }
```

---

## 🔧 Development

### Run Tests
```bash
cargo test --workspace
```

### Check Linting
```bash
cargo clippy --workspace
```

### Format Code
```bash
cargo fmt --all
```

---

## 📝 Notes

- **Rate Limiting:** WeWorkRemotely may block aggressive scraping. Add delays between requests if needed.
- **Index Persistence:** The `search_index/` directory persists between runs. Delete it to force re-indexing.
- **Schema Changes:** If you modify the Tantivy schema, delete `search_index/` before restarting the server.

---

## 📄 License

MIT License - See LICENSE file for details.
