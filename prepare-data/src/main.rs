use std::path::PathBuf;

use meteo::Report;
use scraper::{Html, Selector};

#[tokio::main]
async fn main() {
    let base_url = "http://meteo.lyc-chamson-levigan.ac-montpellier.fr/meteo/";
    let main_page = format!("{base_url}?page=releve");

    println!("Downloading the main page at: {main_page}");
    let response = reqwest::get(&main_page).await.unwrap();
    let body = response.bytes().await.unwrap();
    let (body, _, _) = encoding_rs::WINDOWS_1252.decode(&body);
    let document = Html::parse_document(&body);
    let selector = Selector::parse("#gauche select option").unwrap();
    let files: Vec<_> = document
        .select(&selector)
        .filter_map(|el| el.attr("value").map(|attr| (el.inner_html(), attr))) // skip everything that doesn't contains a value
        .filter(|(_name, url)| !url.is_empty()) // skip the empty values
        .map(|(name, url)| (name, format!("{base_url}{url}")))
        .collect();

    println!("Downloading all the reports");

    let mut reports = Vec::new();
    let mut read_dir = tokio::fs::read_dir("../assets/reports/raw").await.unwrap();
    while let Some(dir) = read_dir.next_entry().await.unwrap() {
        reports.push(dir.path());
    }
    let mut handles = Vec::new();
    for (name, url) in files {
        handles.push(tokio::spawn(handle_report(
            reports.clone(),
            name,
            url.to_string(),
        )));
    }

    // let mut reports = Vec::new();
    for handle in handles {
        if let Ok(_report) = handle.await {
            // reports.push(report);
        }
    }
}

async fn handle_report(reports: Vec<PathBuf>, name: String, url: String) -> Option<Report> {
    let filename = PathBuf::from(sanitize(&name));
    let path = PathBuf::from("../assets/reports/raw/").join(filename);
    // We **always** wants to update the last two reports
    if !url.contains("NOAA") && reports.contains(&path) {
        return None;
    }
    println!("Downloading the report {name}");
    let response = reqwest::get(url).await.unwrap();
    let body = response.bytes().await.unwrap();
    println!("Downloaded the report {name}");
    let (body, _, _) = encoding_rs::WINDOWS_1252.decode(&body);
    // replace the useless crlf separator
    let body = body.replace("\r\n", "\n");
    tokio::fs::write(path, body.as_bytes()).await.unwrap();
    println!("Wrote the report on disk");
    Some(body.parse::<meteo::Report>().unwrap())
}

fn sanitize(s: &str) -> String {
    s.replace("/", "-")
}
