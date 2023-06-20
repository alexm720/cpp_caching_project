//!
//! This file demonstrates the mechanics of accessing the remote server code. Combined with the
//! mock yaml in the resources directory, it allows you to make localhost calls, and see how to
//! deserialize the JSON responses from the server.
//!
//! However, it does NOT implement any caching. The tests only aspirationally mention that the
//! remote server should not be accessed by calls that should be cached according to the specs, but
//! the implementation here does not provide the caching.
//!
//! The code is written in Rust only for a trivial sample implementation to clarify expectations as
//! well as give you a taste of this powerful modern systems language. You are welcome to translate
//! the "mechanics" aspect to your systems language of choice and then add the caching layer on
//! top.
//!
use crate::OpenWeatherCache;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use datetime::Instant;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TWO_HOURS: i64 = 2 * 60 * 60;
const ONE_DAY: i64 = 24 * 60 * 60;
const MINUTE: i64 = 60;
const FIVE_MINUTES: i64 = 5 * 60;
const ONE_HOUR: i64 = 60 * 60;

// https://openweathermap.org/forecast5
// structures have been created for all the returned data, even though the main piece of interest
// is the `temp` field
#[derive(Debug, Deserialize, Serialize)]
struct APIResponseMain {
    temp: f64, // this field is useful for the programming challenge
    feels_like: f64,
    temp_min: f64,
    temp_max: f64,
    pressure: f64,
    sea_level: f64,
    grnd_level: f64,
    humidity: f64,
    temp_kf: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct APIResponseWeather {
    id: u32,
    main: String,
    description: String,
    icon: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct APIResponseClouds {
    all: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct APIResponseWind {
    speed: f32,
    deg: u32,
    gust: f32,
}

#[derive(Debug, Deserialize, Serialize)]
struct APIResponseSys {
    pod: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct APIResponseElem {
    dt: i64, // this field is useful for the programming challenge
    main: APIResponseMain,
    weather: Vec<APIResponseWeather>,
    clouds: APIResponseClouds,
    wind: APIResponseWind,
    visibility: u32,
    pop: f32,
    sys: APIResponseSys,
    dt_txt: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct APIResponse {
    cod: String,
    message: u32,
    cnt: u32,
    list: Vec<APIResponseElem>,
}

// Simple struct wrapping a geo location
pub struct NonCachingClient {
    lat: f64,
    long: f64,
}

impl NonCachingClient {
    // Makes the remote call. Error handling is simple, no attempt is made to distinguish between
    // auth errors, network errors or deserialization errors.
    pub(crate) async fn get_remote_data_five_day_forecast(&self) -> Result<APIResponse> {
        let http_client = Client::builder().build().unwrap();
        #[cfg(feature = "use_remote_api")]
        let url = format!(
            "http://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&appid=seekrit",
            self.lat, self.long
        );

        #[cfg(not(feature = "use_remote_api"))]
        let url = format!(
            "http://localhost:50000/data/2.5/forecast?lat={}&lon={}",
            self.lat, self.long
        );
        match http_client.get(url).send().await?.json().await {
            Ok(resp) => Ok(resp),
            Err(err) => Err(anyhow!(
                "error from get_remote_data_five_day_forecast {:?}",
                err
            )),
        }
    }
}

#[async_trait]
impl OpenWeatherCache for NonCachingClient {
    fn new(lat: f64, long: f64) -> Self {
        Self { lat, long }
    }

    // the key part of the functionality
    // NB: no caching in this sample implementation
    async fn query(&self, start: Instant, end: Instant) -> Result<Vec<Option<f64>>> {
        // NB: using the 5 day forecast, independent of the requested time ranges. Assumption is
        // that use case is focusing on this time range only.
        let remote_data = self.get_remote_data_five_day_forecast().await?;
        // simply assume that the server will send valid data
        if remote_data.list.is_empty() {
            return Err(anyhow!("returned data list is empty"));
        }
        // validate user input
        if start > end {
            return Err(anyhow!("start {:?} is greater than end {:?}", start, end));
        } else if start == end {
            return Ok(vec![]);
        }

        let start_secs = start.seconds();
        let end_secs = end.seconds();
        let requested_range = end_secs - start_secs;

        // you can assume that the server response will have the timestamps sorted
        let min_dt = remote_data.list[0].dt;
        let max_dt = remote_data.list[remote_data.list.len() - 1].dt;
        let available_range = min_dt..max_dt;

        if !available_range.contains(&start_secs) || !available_range.contains(&end_secs) {
            return Err(anyhow!(
                "returned data range {:?} is smaller than requested range start {} end {}",
                available_range,
                start_secs,
                end_secs
            ));
        }
        let mut returned_data_map = BTreeMap::<i64, f64>::new();
        for e in remote_data.list {
            returned_data_map.insert(e.dt, e.main.temp);
        }

        // business logic of the programming challenge for granularity
        // minute for less than 2 hours
        // 5 minutes for less than 1 day
        // 1 hour otherwise
        let granularity = if requested_range < TWO_HOURS {
            MINUTE
        } else if requested_range < ONE_DAY {
            FIVE_MINUTES
        } else {
            ONE_HOUR
        };

        let mut i = start_secs;
        let mut ret = Vec::<Option<f64>>::new();
        while i < end_secs {
            i = i + granularity;
            // Simple interpolation: for the requested ts, find the closest data point
            // One can imagine this step requiring significant computation if this is based on
            // trendlines and forecasts between the "known" data points
            match returned_data_map.range(..i).next_back() {
                None => ret.push(None),
                Some((_, temp)) => ret.push(Some(*temp)),
            }
        }

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        non_caching_client::{NonCachingClient, ONE_HOUR},
        OpenWeatherCache,
    };
    use claim::{assert_err, assert_ok};
    use datetime::Instant;

    const SAMPLE_DATA_START: i64 = 1659722400;

    #[tokio::test]
    async fn client_expected_cities() {
        let client1 = NonCachingClient::new(45.62, -122.67);
        assert_ok!(client1.get_remote_data_five_day_forecast().await);
        let client2 = NonCachingClient::new(47.36, -122.19);
        assert_ok!(client2.get_remote_data_five_day_forecast().await);
    }

    #[tokio::test]
    async fn client_unexpected_city() {
        let client1 = NonCachingClient::new(0.0, 0.0);
        assert_err!(client1.get_remote_data_five_day_forecast().await);
    }

    #[tokio::test]
    async fn demonstrate_interpolation() {
        let client = NonCachingClient::new(47.36, -122.19);
        let start = SAMPLE_DATA_START;
        let end = start + 25 * ONE_HOUR;
        let data = client
            .query(Instant::at(start), Instant::at(end))
            .await
            .unwrap();
        // 25 hours in 1 hour intervals
        assert_eq!(25, data.len());
        // the first three data points are the same, because we are downsampling from a 3 hour
        // granularity to a 1 hour granularty by simply repeating
        assert_eq!(Some(290.18), data[0]);
        assert_eq!(Some(290.18), data[1]);
        assert_eq!(Some(290.18), data[2]);
    }

    #[tokio::test]
    async fn expect_single_remote_call() {
        let client = NonCachingClient::new(47.36, -122.19);
        let start = 1659722400;
        let end = start + 3 * 60 * 60;

        // NB: simply memoizing the `query` call will make this test pass
        for _ in 0..5 {
            let data = client
                .query(Instant::at(start), Instant::at(end))
                .await
                .unwrap();
            assert_eq!(36, data.len()); // 3 hours in 5 minute intervals
        }
    }

    #[tokio::test]
    async fn expect_single_remote_call_overlapping_ranges() {
        let client = NonCachingClient::new(47.36, -122.19);
        let start = SAMPLE_DATA_START;
        let end1 = start + 1 * ONE_HOUR;
        let end3 = start + 3 * ONE_HOUR;
        let end25 = start + 25 * ONE_HOUR;

        // NB: simply memoizing the `query` call will NOT make this test pass
        for _ in 0..1 {
            let data = client
                .query(Instant::at(start), Instant::at(end25))
                .await
                .unwrap();
            assert_eq!(25, data.len()); // 25 hours in 1 hour intervals

            let data = client
                .query(Instant::at(start), Instant::at(end3))
                .await
                .unwrap();
            assert_eq!(36, data.len()); // 3 hours in 5 minute intervals

            let data = client
                .query(Instant::at(start), Instant::at(end1))
                .await
                .unwrap();
            assert_eq!(60, data.len()); // 1 hour in 1 minute intervals
        }
    }
}
