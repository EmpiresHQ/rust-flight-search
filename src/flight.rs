use chrono::{Duration, NaiveDateTime};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use dashmap::DashMap;

use crate::airport::Airport;

#[derive(Clone, Debug)]
pub struct FlightEdge {
    pub flight_id: usize,
    pub to: Arc<RwLock<Airport>>,
    pub from: Arc<RwLock<Airport>>,
    pub cost: i32,
    pub arrive_at: NaiveDateTime,
    pub depart_at: NaiveDateTime,
}

impl Hash for FlightEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.flight_id.hash(state);
    }
}

// Custom Eq: Compare only `flight_id`
impl PartialEq for FlightEdge {
    fn eq(&self, other: &Self) -> bool {
        self.flight_id == other.flight_id
    }
}
impl Eq for FlightEdge {}

#[derive(Clone, Debug)]
pub struct FlightDTO {
    pub flight_id: usize,
    pub from: usize,
    pub to: usize,
    pub cost: i32,
    pub arrival_date: String,
    pub departure_date: String,
}
pub struct FlightsContainer {
    pub flights: DashMap<usize, Arc<FlightEdge>>,
}

impl FlightDTO {
    // Helper function to fix dates with 24:00:00 time
    fn fix_datetime_format(datetime_str: &str) -> String {
        if datetime_str.ends_with(" 24:00:00") {
            // Extract the date part
            if let Some(date_part) = datetime_str.split_whitespace().next() {
                // Parse the date and add one day
                if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                    let next_day = date.succ_opt().unwrap_or(date);
                    return format!("{} 00:00:00", next_day);
                }
            }
        }
        datetime_str.to_string()
    }

    pub fn to_edge(&self, from: Arc<RwLock<Airport>>, to: Arc<RwLock<Airport>>) -> FlightEdge {
        // println!("Creating flight edge from {} to {} at {}", from.read().unwrap().name, to.read().unwrap().name, &self.arrival_date);

        // Fix dates that may have "24:00:00" time format
        let fixed_arrival_date = Self::fix_datetime_format(&self.arrival_date);
        let fixed_departure_date = Self::fix_datetime_format(&self.departure_date);

        let mut arrival_date =
            NaiveDateTime::parse_from_str(&fixed_arrival_date, "%Y-%m-%d %H:%M:%S").unwrap();
        let departure_date =
            NaiveDateTime::parse_from_str(&fixed_departure_date, "%Y-%m-%d %H:%M:%S").unwrap();

        if arrival_date < departure_date {
            arrival_date = arrival_date + Duration::days(1);
        }

        FlightEdge {
            flight_id: self.flight_id,
            to,
            from,
            cost: self.cost,
            arrive_at: arrival_date,
            depart_at: departure_date,
        }
    }

    pub fn departure_date(&self) -> NaiveDateTime {
        let fixed_departure_date = Self::fix_datetime_format(&self.departure_date);
        NaiveDateTime::parse_from_str(&fixed_departure_date, "%Y-%m-%d %H:%M:%S").unwrap()
    }
}

impl FlightsContainer {
    pub fn new() -> Self {
        FlightsContainer {
            flights: DashMap::new(),
        }
    }
    pub fn add_flight(&mut self, flight: FlightEdge) -> Arc<FlightEdge> {
        let flight = Arc::new(flight);
        self.flights.insert(flight.flight_id, flight.clone());
        flight
    }

    pub fn get_flight(&self, flight_id: usize) -> Option<Arc<FlightEdge>> {
        if let Some(flight) = self.flights.get(&flight_id) {
            Some(flight.clone())
        } else {
            None
        }
    }

    pub fn remove_flight(&mut self, flight_id: usize) -> Result<(), &str> {
        if self.flights.remove(&flight_id).is_some() {
            Ok(())
        } else {
            Err("Flight not found.")
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlightEdgeWrapper(pub Arc<FlightEdge>);

impl FlightEdgeWrapper {
    pub fn new(flight: Arc<FlightEdge>) -> Self {
        FlightEdgeWrapper(flight)
    }

    pub fn flight(&self) -> Arc<FlightEdge> {
        self.0.clone()
    }
}

impl PartialEq for FlightEdgeWrapper {
    fn eq(&self, other: &Self) -> bool {
        // Compare based on flight cost
        self.0.clone().cost == other.0.clone().cost
    }
}

impl Eq for FlightEdgeWrapper {}

impl PartialOrd for FlightEdgeWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse the order: lower cost is considered "greater" for BinaryHeap
        other.0.clone().cost.partial_cmp(&self.0.clone().cost)
    }
}

impl Ord for FlightEdgeWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.clone().cost.cmp(&self.0.clone().cost)
    }
}
