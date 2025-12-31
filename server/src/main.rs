//! Job Search Engine Server
//!
//! Provides full-text search over scraped job listings using Tantivy
//! and exposes a REST API using Axum.

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use common::Job;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tantivy::{
    Index, IndexReader, ReloadPolicy,
    collector::TopDocs,
    query::QueryParser,
    schema::{INDEXED, IntOptions, STORED, Schema, TEXT},
};

/// Search result returned by the API
#[derive(Debug, Serialize)]
struct SearchResult {
    title: String,
    company: String,
    score: f32,
}

/// API response wrapper
#[derive(Debug, Serialize)]
struct SearchResponse {
    query: String,
    total_results: usize,
    results: Vec<SearchResult>,
}

/// Query parameters for search endpoint
#[derive(Debug, Deserialize)]
struct SearchParams {
    q: Option<String>,
}

/// Shared application state
struct AppState {
    index_reader: IndexReader,
    query_parser: QueryParser,
    schema: Schema,
}

/// Builds the Tantivy schema for job indexing
fn build_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // Title: searchable and stored (returned in results)
    schema_builder.add_text_field("title", TEXT | STORED);

    // Company: searchable and stored
    schema_builder.add_text_field("company", TEXT | STORED);

    // Description: searchable but not stored (saves space)
    schema_builder.add_text_field("description", TEXT);

    // Salary: indexed for filtering, but as i64 field
    let int_options = IntOptions::default().set_indexed();
    schema_builder.add_i64_field("salary_min", int_options);

    schema_builder.build()
}

/// Creates or opens the search index and indexes all jobs
fn create_index(jobs: &[Job]) -> tantivy::Result<Index> {
    let schema = build_schema();
    let index_path = Path::new("search_index");

    // Create directory if needed
    if !index_path.exists() {
        fs::create_dir_all(index_path)?;
    }

    // Create or open index
    let index = if index_path.join("meta.json").exists() {
        println!("📂 Opening existing index...");
        Index::open_in_dir(index_path)?
    } else {
        println!("📝 Creating new index...");
        Index::create_in_dir(index_path, schema.clone())?
    };

    // Get field handles
    let title_field = index.schema().get_field("title").unwrap();
    let company_field = index.schema().get_field("company").unwrap();
    let description_field = index.schema().get_field("description").unwrap();
    let salary_field = index.schema().get_field("salary_min").unwrap();

    // Create index writer with 50MB heap
    let mut index_writer = index.writer(50_000_000)?;

    // Clear existing documents (for fresh re-indexing)
    index_writer.delete_all_documents()?;

    println!("📊 Indexing {} jobs...", jobs.len());

    // Index each job
    for job in jobs {
        let mut doc = tantivy::Document::new();
        doc.add_text(title_field, &job.title);
        doc.add_text(company_field, &job.company);
        doc.add_text(description_field, &job.description);
        if let Some(salary) = job.salary_min {
            doc.add_i64(salary_field, salary);
        }
        index_writer.add_document(doc)?;
    }

    // Commit changes
    index_writer.commit()?;
    println!("✅ Indexing complete!");

    Ok(index)
}

/// Handler for GET /search?q=<keywords>
async fn search_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Json<SearchResponse> {
    let query_str = params.q.unwrap_or_default();

    if query_str.is_empty() {
        return Json(SearchResponse {
            query: query_str,
            total_results: 0,
            results: vec![],
        });
    }

    // Parse the query
    let query = match state.query_parser.parse_query(&query_str) {
        Ok(q) => q,
        Err(_) => {
            return Json(SearchResponse {
                query: query_str,
                total_results: 0,
                results: vec![],
            });
        }
    };

    // Get field handles for retrieving stored fields
    let title_field = state.schema.get_field("title").unwrap();
    let company_field = state.schema.get_field("company").unwrap();

    // Search the index
    let searcher = state.index_reader.searcher();
    let top_docs = match searcher.search(&query, &TopDocs::with_limit(10)) {
        Ok(docs) => docs,
        Err(_) => {
            return Json(SearchResponse {
                query: query_str,
                total_results: 0,
                results: vec![],
            });
        }
    };

    // Collect results
    let mut results = Vec::new();
    for (score, doc_address) in top_docs {
        if let Ok(retrieved_doc) = searcher.doc(doc_address) {
            let title = retrieved_doc
                .get_first(title_field)
                .and_then(|v| v.as_text())
                .unwrap_or("Unknown")
                .to_string();

            let company = retrieved_doc
                .get_first(company_field)
                .and_then(|v| v.as_text())
                .unwrap_or("Unknown")
                .to_string();

            results.push(SearchResult {
                title,
                company,
                score,
            });
        }
    }

    Json(SearchResponse {
        query: query_str,
        total_results: results.len(),
        results,
    })
}

/// Handler for GET / (root)
async fn root_handler() -> &'static str {
    "🔍 Job Search Engine API\n\nEndpoints:\n  GET /search?q=<keywords> - Search for jobs\n\nExample:\n  curl 'http://127.0.0.1:3000/search?q=rust developer'"
}

#[tokio::main]
async fn main() {
    println!("🚀 Starting Job Search Engine Server...\n");

    // Load jobs from JSON file
    let jobs_path = Path::new("data/jobs.json");

    let jobs: Vec<Job> = if jobs_path.exists() {
        println!("📂 Loading jobs from {:?}", jobs_path);
        let content = fs::read_to_string(jobs_path).expect("Failed to read jobs.json");
        serde_json::from_str(&content).expect("Failed to parse jobs.json")
    } else {
        println!("⚠️  No jobs.json found. Run the scraper first!");
        println!("   cargo run -p scraper");
        vec![]
    };

    println!("📊 Loaded {} jobs\n", jobs.len());

    // Create search index
    let index = create_index(&jobs).expect("Failed to create search index");
    let schema = index.schema();

    // Create index reader
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into()
        .expect("Failed to create index reader");

    // Create query parser for title and description fields
    let title_field = schema.get_field("title").unwrap();
    let description_field = schema.get_field("description").unwrap();
    let query_parser = QueryParser::for_index(&index, vec![title_field, description_field]);

    // Create shared state
    let state = Arc::new(AppState {
        index_reader: reader,
        query_parser,
        schema,
    });

    // Build router
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/search", get(search_handler))
        .with_state(state);

    // Start server
    let addr = "127.0.0.1:3000";
    println!("🌐 Server running at http://{}", addr);
    println!("   Try: curl 'http://{}/search?q=developer'\n", addr);

    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
