use crate::airport::{Airport, AirportsContainer};
use crate::flight::FlightEdge;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::sync::{Arc, RwLock};
use tokio::task;

#[derive(Clone)]
pub struct SearchQuery {
    pub from: usize,
    pub to: usize,
    pub date: String,
    pub hops: usize,
    pub results: usize,
}

pub struct Search {
    pub airports: Arc<RwLock<AirportsContainer>>,
}

#[derive(Clone, Debug)]
pub struct PathState {
    cost: i32,
    current: Arc<FlightEdge>,
    path: Vec<Arc<FlightEdge>>,
}
impl PathState {
    pub fn readable_path(&self) -> Vec<String> {
        let mut paths = self
            .path
            .iter()
            .map(|x| {
                let edge = x.clone();
                format!(
                    "Flight {} from {} to {}, from {} to {}",
                    edge.flight_id,
                    edge.from.read().unwrap().name,
                    edge.to.read().unwrap().name,
                    edge.depart_at,
                    edge.arrive_at
                )
            })
            .collect::<Vec<String>>();
        paths.push(format!("Total cost: {}", self.cost));
        paths
    }
}

impl Search {
    pub fn new(airports: AirportsContainer) -> Self {
        Search {
            airports: Arc::new(RwLock::new(airports)),
        }
    }

    pub fn find(&self, query: SearchQuery) -> Vec<PathState> {
        let airports_guard = self.airports.read().unwrap();
        let airports = &airports_guard.airports;
        
        let from = match airports.get(&query.from) {
            Some(airport) => airport.clone(),
            None => return vec![],
        };
        let to = match airports.get(&query.to) {
            Some(airport) => airport.clone(),
            None => return vec![],
        };
        let date = NaiveDate::parse_from_str(&query.date, "%Y-%m-%d").unwrap();
        let mut found: Vec<PathState> = self
            .traverse(
                from,
                to,
                date.and_hms_opt(0, 0, 0).unwrap(),
                query.hops,
                query.results,
                &airports_guard,
            )
            .into_sorted_vec()
            .into_iter()
            .map(|Reverse(x)| x)
            .collect();
        found.reverse();
        return found;
    }

    pub async fn find_async(&self, query: SearchQuery) -> Vec<PathState> {
        let airports_arc = Arc::clone(&self.airports);
        
        let results = task::spawn_blocking(move || {
            let search = Search { airports: airports_arc };
            search.find(query)
        })
        .await
        .unwrap_or_default();
        
        results
    }

    fn traverse(
        &self,
        source: Arc<RwLock<Airport>>,
        target: Arc<RwLock<Airport>>,
        date: NaiveDateTime,
        k: usize,
        total: usize,
        airports_container: &AirportsContainer,
    ) -> BinaryHeap<Reverse<PathState>> {
        let num_nodes = airports_container
            .flights_container
            .read()
            .unwrap()
            .flights
            .len();
        let mut heap = BinaryHeap::new();
        let mut count = vec![0; num_nodes];

        let flights = source
            .read()
            .unwrap()
            .flights_between(date, Some(date + Duration::hours(24)));
        for flight in flights {
            let edge = flight.clone();
            if edge.depart_at < date {
                continue;
            }
            let new_state = PathState {
                cost: edge.cost,
                current: Arc::clone(&flight),
                path: vec![Arc::clone(&flight)],
            };
            heap.push(new_state);
        }

        let mut results = BinaryHeap::new();

        while let Some(state) = heap.pop() {
            let curr = state.current.clone();
            let cur_id = curr.flight_id;
            count[cur_id] += 1;

            if curr.to.read().unwrap().id == target.read().unwrap().id {
                results.push(Reverse(state.clone()));
                if results.len() == total {
                    return results;
                }
            }
            if state
                .path
                .iter()
                .find(|x| x.from.read().unwrap().id == curr.to.read().unwrap().id)
                .is_some()
            {
                continue;
            }
            if count[cur_id] > k {
                continue;
            }

            let start_date = curr.arrive_at + Duration::minutes(15);
            let end_date = Some(date + Duration::hours(24));

            if start_date > end_date.unwrap() {
                continue;
            }
            let dest_airport = state.current.clone().to.clone();
            let flights = dest_airport
                .read()
                .unwrap()
                .flights_between(start_date, end_date);

            for flight in &flights {
                let edge = flight.clone();
                if edge.arrive_at > date + Duration::hours(48) {
                    continue;
                }
                if edge.to.read().unwrap().id == source.read().unwrap().id {
                    continue;
                }

                let new_cost = state.cost + edge.cost;
                let mut new_path = state.path.clone();
                new_path.push(Arc::clone(flight));
                let new_state = PathState {
                    cost: new_cost,
                    current: Arc::clone(flight),
                    path: new_path,
                };
                heap.push(new_state);
            }
        }
        results
    }
}

impl PartialEq for PathState {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for PathState {}
impl PartialOrd for PathState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.cost.partial_cmp(&self.cost)
    }
}
impl Ord for PathState {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}
