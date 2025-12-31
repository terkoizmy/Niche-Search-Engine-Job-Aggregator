//! WeWorkRemotely Job Scraper
//!
//! Scrapes backend programming jobs from WeWorkRemotely.com
//! and saves them to data/jobs.json

use common::Job;
use regex::Regex;
use scraper::{Html, Selector};
use std::fs;
use std::path::Path;

/// Extracts the minimum salary from a raw salary string using regex.
/// Looks for numbers and returns the first one found (likely the minimum).
fn extract_salary(salary_raw: &str) -> Option<i64> {
    // Match numbers that may have commas (e.g., "50,000" or "50000")
    let re = Regex::new(r"\$?(\d{1,3}(?:,\d{3})*|\d+)").ok()?;

    for cap in re.captures_iter(salary_raw) {
        if let Some(matched) = cap.get(1) {
            // Remove commas and parse as i64
            let clean_number: String = matched
                .as_str()
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            if let Ok(num) = clean_number.parse::<i64>() {
                // Only consider reasonable salary values (at least 1000)
                if num >= 1000 {
                    return Some(num);
                }
            }
        }
    }
    None
}

fn main() {
    println!("🔍 Starting WeWorkRemotely Job Scraper...\n");

    let urls = vec![
        "https://weworkremotely.com/remote-software-developer-jobs",
        "https://weworkremotely.com/categories/remote-full-stack-programming-jobs",
        "https://weworkremotely.com/categories/remote-back-end-programming-jobs",
        "https://weworkremotely.com/categories/remote-front-end-programming-jobs",
    ];

    // Define selectors once (reused for all pages)
    let job_selector = Selector::parse("li.feature, .new-listing-container").unwrap();
    let title_selector = Selector::parse(".new-listing__header__title").unwrap();
    let company_selector = Selector::parse(".new-listing__company-name").unwrap();
    let region_selector = Selector::parse(".new-listing__company-headquarters").unwrap();
    let link_selector = Selector::parse(".listing-link--unlocked, ._blank").unwrap();

    let mut jobs: Vec<Job> = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Iterate through all URLs
    for url in &urls {
        println!("📡 Fetching jobs from: {}", url);

        let response = match reqwest::blocking::get(*url) {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("❌ Failed to fetch URL {}: {}", url, e);
                continue; // Skip to next URL instead of stopping
            }
        };

        let html_content = match response.text() {
            Ok(text) => text,
            Err(e) => {
                eprintln!("❌ Failed to read response body: {}", e);
                continue;
            }
        };

        println!("✅ Fetched {} bytes from {}", html_content.len(), url);

        // Parse HTML document
        let document = Html::parse_document(&html_content);

        // Iterate through job listings
        for element in document.select(&job_selector) {
            // Extract title
            let title = element
                .select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| "Unknown Title".to_string());

            // Extract company name
            let company = element
                .select(&company_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| "Unknown Company".to_string());

            // Extract location/region
            let location = element
                .select(&region_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| "Remote".to_string());

            // Extract job URL
            let job_url = element
                .select(&link_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(|href| {
                    if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://weworkremotely.com{}", href)
                    }
                })
                .unwrap_or_else(|| "No URL".to_string());

            // Skip duplicates (same job may appear on multiple category pages)
            if seen_urls.contains(&job_url) {
                continue;
            }
            seen_urls.insert(job_url.clone());

            // Get full text for salary extraction
            let full_text = element.text().collect::<String>();
            let salary_raw = full_text.clone();
            let salary_min = extract_salary(&salary_raw);

            // Create Job struct
            let job = Job {
                title,
                company,
                location,
                description: salary_raw.trim().replace('\n', " ").replace("  ", " "),
                salary_min,
                url: job_url,
            };

            // Only add if we have a valid title
            if job.title != "Unknown Title" && !job.title.is_empty() {
                println!("📋 Found: {} at {}", job.title, job.company);
                jobs.push(job);
            }
        }

        println!(""); // Blank line between URL fetches
    }

    println!("📊 Total unique jobs found: {}", jobs.len());

    // Create data directory if it doesn't exist
    let data_dir = Path::new("data");
    if !data_dir.exists() {
        fs::create_dir_all(data_dir).expect("Failed to create data directory");
        println!("📁 Created 'data' directory");
    }

    // Save to JSON file
    let json_output =
        serde_json::to_string_pretty(&jobs).expect("Failed to serialize jobs to JSON");
    let output_path = data_dir.join("jobs.json");

    fs::write(&output_path, &json_output).expect("Failed to write jobs.json");

    println!("💾 Saved {} jobs to {:?}", jobs.len(), output_path);
    println!("\n✨ Scraping complete!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_salary_with_dollar_sign() {
        assert_eq!(extract_salary("$50,000 - $70,000"), Some(50000));
    }

    #[test]
    fn test_extract_salary_without_dollar_sign() {
        assert_eq!(extract_salary("Salary: 60000 USD"), Some(60000));
    }

    #[test]
    fn test_extract_salary_no_salary() {
        assert_eq!(extract_salary("Competitive salary"), None);
    }

    #[test]
    fn test_extract_salary_with_k_notation() {
        // This would need enhancement to handle "50k" notation
        assert_eq!(extract_salary("$120,000/year"), Some(120000));
    }
}
