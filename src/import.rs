use std::error::Error;
use std::fs::File;

use crate::airport::{Airport, AirportsContainer};
use crate::flight::FlightDTO;

pub trait FlightImporter {
    fn import_flights(&self, file_path: &str) -> Result<usize, Box<dyn Error>>;
}

pub struct CsvFlightImporter<'a> {
    airports_container: &'a AirportsContainer,
}

impl<'a> CsvFlightImporter<'a> {
    pub fn new(airports_container: &'a AirportsContainer) -> Self {
        CsvFlightImporter {
            airports_container,
        }
    }

    // Parse time from HHMM format and combine with flight date
    fn format_datetime(flight_date: &str, time_str: &str) -> String {
        if time_str.len() < 2 {
            return format!("{} 00:00:00", flight_date);
        }

        let time_digits = time_str.trim();
        let (hours, minutes) = if time_digits.len() <= 2 {
            (time_digits, "00")
        } else if time_digits.len() == 3 {
            ("0", &time_digits[0..1])
        } else {
            // Format HHMM -> HH:MM
            let len = time_digits.len();
            (&time_digits[0..len-2], &time_digits[len-2..])
        };

        format!("{} {}:{}:00", flight_date, hours, minutes)
    }
}

impl<'a> FlightImporter for CsvFlightImporter<'a> {
    fn import_flights(&self, file_path: &str) -> Result<usize, Box<dyn Error>> {
        let file = File::open(file_path)?;
        let mut rdr = csv::Reader::from_reader(file);
        let mut flights_added = 0;
        let mut next_flight_id = 0;

        // Get the max flight ID to avoid duplicates
        if let Ok(flights_container) = self.airports_container.flights_container.read() {
            next_flight_id = flights_container.flights.len();
        }

        for result in rdr.records() {
            let record = result?;
            
            // Skip if the record doesn't have enough fields
            // if record.len() < 85 {
            //     continue;
            // }

            // Extract flight data from CSV
            // OriginAirportID (column 20)
            let origin_id = match record[20].parse::<usize>() {
                Ok(id) => id,
                Err(_) => continue,
            };

            // DestAirportID (column 30)
            let dest_id = match record[29].parse::<usize>() {
                Ok(id) => id,
                Err(_) => continue,
            };

            if dest_id == origin_id {
                println!("Skipping flight with same origin and destination: {}", origin_id);
                continue;
            }

            // Origin and destination airport codes
            let origin_code = record[23].to_string();
            let dest_code = record[32].to_string();

            // println!("adding flight: {}, {}, {}, {}", origin_id, dest_id, origin_code, dest_code);
            
            // Flight date (column 6)
            let flight_date = record[5].trim();
            if flight_date.is_empty() {
                continue;
            }

            // CRSDepTime (column 39) and CRSArrTime (column 47)
            let dep_time = record[38].trim();
            let arr_time = record[49].trim();
            // println!("dep_time: {}, arr_time: {}", dep_time, arr_time);
            if dep_time.is_empty() || arr_time.is_empty() {
                continue;
            }

            // Distance (column 86)
            let distance = match record[63].parse::<i32>() {
                Ok(d) => d,
                Err(_) => 0,
            };

            // Calculate cost based on distance (simple approach)
            let cost = distance;

            // Create properly formatted date strings
            let departure_date = Self::format_datetime(flight_date, dep_time);
            let arrival_date = Self::format_datetime(flight_date, arr_time);
            // println!("adding flight: {}, {}, {}, {}, {}, {}", origin_id, dest_id, arrival_date, departure_date, origin_code, dest_code);

            // let has_airport = self.airports_container.has_airport(origin_id);
            // println!("has_airport: {}, {}", origin_id, has_airport);
            // Ensure both airports exist
            if !self.airports_container.has_airport(origin_id) {
                let airport = Airport {
                    id: origin_id,
                    name: origin_code,
                    outgoing: std::collections::BTreeMap::new(),
                };
                self.airports_container.add_airport(airport);
            }

            if !self.airports_container.has_airport(dest_id) {
                let airport = Airport {
                    id: dest_id,
                    name: dest_code,
                    outgoing: std::collections::BTreeMap::new(),
                };
                self.airports_container.add_airport(airport);
            }

            // Create and add the flight
            let flight_dto = FlightDTO {
                flight_id: next_flight_id,
                from: origin_id,
                to: dest_id,
                cost,
                arrival_date,
                departure_date,
            };

            self.airports_container.add_flight(flight_dto);
            next_flight_id += 1;
            flights_added += 1;
        }

        Ok(flights_added)
    }
}
