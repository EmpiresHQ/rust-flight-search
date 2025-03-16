use std::path::Path;
use sysinfo::System;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod airport;
use airport::AirportsContainer;
mod flight;
mod search;
use search::{Search, SearchQuery};
mod import;
use import::{CsvFlightImporter, FlightImporter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_test=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let mut sys = System::new_all();
    sys.refresh_all();

    let airports = AirportsContainer::new();

    // Try to import flights from CSV if the file exists
    let flights_file = "data/flights.csv";
    if Path::new(flights_file).exists() {
        let importer = CsvFlightImporter::new(&airports);
        match importer.import_flights(flights_file) {
            Ok(count) => println!("Imported {} flights from CSV", count),
            Err(e) => {
                println!("Failed to import flights from CSV: {}", e);
            }
        }
    } else {
        eprintln!("Flights file not found: {}", flights_file);
        std::process::exit(1)
    }

    let search = Search::new(airports);

    let query = SearchQuery {
        from: 14576,
        to: 14689,
        date: "2024-01-14".to_string(),
        hops: 3,
        results: 10,
    };
    println!(
        "Searching for flights from {} to {} on {}",
        query.from, query.to, query.date
    );

    let start = std::time::Instant::now();
    // Run the search asynchronously
    let results = search.find_async(query).await;

    println!("Search completed in: {:?}", start.elapsed());

    for result in results {
        println!("---- {:#?}", result.readable_path());
    }
    
    let pid = sysinfo::get_current_pid().expect("Failed to get current PID");

    // Lookup the process using the PID.
    if let Some(process) = sys.process(pid) {
        // process.memory() returns the memory usage in bytes.
        println!("Memory usage: {} Mb", process.memory() / 1024 / 1024);
    }
}
