use chrono::{Duration, NaiveDateTime};
use std::collections::{BTreeMap, BinaryHeap};
use std::error::Error;
use std::fs::File;
use std::sync::{Arc, RwLock};

use crate::flight::{self, FlightDTO};
use flight::{FlightEdge, FlightEdgeWrapper, FlightsContainer};
use dashmap::DashMap;

pub enum AirportAccess {
    Read(Arc<RwLock<Airport>>),
    Write(Arc<RwLock<Airport>>),
    None,
}

#[derive(Clone)]
pub struct AirportsContainer {
    pub airports: DashMap<usize, Arc<RwLock<Airport>>>,
    pub flights_container: Arc<RwLock<flight::FlightsContainer>>,
}

impl AirportsContainer {
    pub fn new() -> Self {
        AirportsContainer {
            airports: DashMap::new(),
            flights_container: Arc::new(RwLock::new(FlightsContainer::new())),
        }
    }

    pub fn remove_flight(&self, flight_id: usize) {
        let flight = self
            .flights_container
            .read()
            .unwrap()
            .get_flight(flight_id)
            .unwrap();
        let from = flight.from.read().unwrap().id;
        if let Some(airport) = self.airports.get(&from) {
            airport
                .write()
                .unwrap()
                .remove_flight(flight_id, flight.depart_at);
        }
    }

    pub fn add_flight(&self, flight: FlightDTO) {
        let airport_from = self.get_airport_ref(flight.from, true);
        let airport_to = match self.get_airport_ref(flight.to, false) {
            AirportAccess::Read(airport) => airport,
            _ => return,
        };
        match airport_from {
            AirportAccess::Write(airport) => {
                let flight_edge = flight.to_edge(airport.clone(), airport_to);
                let flight_ref = self
                    .flights_container
                    .write()
                    .unwrap()
                    .add_flight(flight_edge.clone());
                airport
                    .write()
                    .unwrap()
                    .add_flight(flight_ref, flight.departure_date());
            }
            _ => {}
        }
    }
    pub fn get_airport_ref(&self, airport_id: usize, write: bool) -> AirportAccess {
        match write {
            true => {
                match self.airports.get(&airport_id) {
                    Some(airport) => AirportAccess::Write(airport.clone()),
                    _ => AirportAccess::None,
                }
            }
            false => {
                match self.airports.get(&airport_id) {
                    Some(airport) => AirportAccess::Read(airport.clone()),
                    _ => AirportAccess::None,
                }
            }
        }
    }
    pub fn add_airport(&self, airport: Airport) {
        self.airports.insert(airport.id, Arc::new(RwLock::new(airport.clone())));
    }

    pub fn has_airport(&self, airport_id: usize) -> bool {
        if self.airports.contains_key(&airport_id) {
            return true;
        }
        false
    }

    pub fn load_airports_from_csv(&self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::open(file_path)?;
        let mut rdr = csv::Reader::from_reader(file);

        for result in rdr.records() {
            let record = result?;

            if record.len() >= 2 {
                let id = record[0].parse::<usize>()?;
                let name = record[3].to_string();

                let airport = Airport {
                    id,
                    name,
                    outgoing: BTreeMap::new(),
                };

                self.add_airport(airport);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Airport {
    pub id: usize,
    pub name: String,
    pub outgoing: BTreeMap<NaiveDateTime, BinaryHeap<FlightEdgeWrapper>>,
}

impl Airport {
    fn add_flight(&mut self, flight: Arc<FlightEdge>, departure_date: NaiveDateTime) {
        // Create a new BinaryHeap if it does not exist.
        let found_departure_slot = self.outgoing.contains_key(&departure_date);
        if !found_departure_slot {
            let mut heap = BinaryHeap::new();
            heap.push(FlightEdgeWrapper::new(flight.clone()));
            self.outgoing.insert(departure_date, heap);
        } else if let Some(heap) = self.outgoing.get_mut(&departure_date) {
            heap.push(FlightEdgeWrapper::new(flight.clone()));
        }
    }

    fn remove_flight(&mut self, flight_id: usize, departure_date: NaiveDateTime) {
        if let Some(heap) = self.outgoing.get_mut(&departure_date) {
            heap.retain(|x| x.flight().clone().flight_id != flight_id);
            if heap.len() == 0 {
                self.outgoing.remove(&departure_date);
            }
        }
    }

    pub fn flights_between(
        &self,
        start: NaiveDateTime,
        end: Option<NaiveDateTime>,
    ) -> Vec<Arc<FlightEdge>> {
        let mut flights = vec![];
        let end_date = end.unwrap_or(start + Duration::hours(24));

        // Continue with collecting flights in range
        for (date, heap) in self.outgoing.range(start..=end_date) {
            if date > &end_date {
                break;
            }
            for flight in heap.iter() {
                flights.push(flight.flight().clone());
            }
        }
        flights
    }
}
