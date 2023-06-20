//! Xetdata’s high performance cache is a core piece of our tech stack. Caches show up everywhere
//! there is a need for low latency and the cost of remote calls is significant.
//!
//! For this programming challenge you will build a cache which will enable a charting UX to be
//! built on top of the OpenWeatherMap temperature API.
//!
//! In terms of user stories,
//! 1. As a user, I can initialize an OpenWeatherCache with a latitude and a longitude representing
//!    a geographic location.
//! 2. As a user, I can access the forecasted temperature of my chosen geographic location for a
//!    time range and time granularity with an initialized OpenWeatherCache. The granularity of the
//!    returned temperature data varies as the time range increases in the following manner: (a)
//!    When the time range is less than 2 hours, return the data at a minute's granularity (b) When
//!    the time range is less than a day, return the data at five minute granularity
//!    (c) Return the data with an hour granularity
//!
//!    You are allowed to implement this solution in any system language of your choice. Candidates
//!    have chosen C++, C#, Java, Golang, and Rust.
//!
//!    You will spend face-to-face with your interviewer to make sure that all your questions are
//!    answered. Once you start on the challenge, you will have access to a slack channel in case
//!    you would like to clarify any confusion.
//!
//!    This challenge is typically finished in around 3-4 hours.
//!    What we are looking for in your submitted artifacts
//!      - Implement the functionality. With Rust, you should be able to run
//!      `cargo run -- -d 2days vancouver` and `cargo test`. Choose similar tooling for your chosen
//!      language.
//!      - Documentation explaining your choices: programming language, caching technology,
//!      parallelism and async options, testing approach, etc.

use anyhow::Result;
use async_trait::async_trait;
use datetime::Instant;

pub mod non_caching_client;

#[async_trait]
trait OpenWeatherCache {
    /// Returns an `OpenWeatherCache` given geographic coordinates
    ///
    /// # Arguments
    ///
    /// * `lat`  - latitude
    /// * `long` - longitude
    fn new(lat: f64, long: f64) -> Self;

    /// Returns temperatures in Kelvin (K) given a time ranges. If data is missing for a particular
    /// timeslot, returns None.
    ///
    /// # Arguments
    ///
    /// * `start` - start of time range
    /// * `end`   - end of time range
    ///
    /// `start` and `end` are in the future, with `end` being at most 5 days out.
    ///
    /// `start` and `end` are guaranteed to be aligned with period constraints described in the
    /// user story. i.e. if the chart covers a four week span, start and end will be aligned on
    /// hourly boundaries; if the chart covers a 36 minute span, start and end will be aligned on
    /// one-minute boundaries.
    ///
    /// OpenWeather API has different constraints. They provide a 5 day forecast with 3 hour
    /// granularity. Your code will be responsible for converting the remote data into this
    /// API's expectations.
    /// hourly forecast for 48 hours, daily forecast for 8 days. This API will average the
    /// OpenWeather data or downsample it to make the translation in expected periods work.
    ///
    /// It is assumed that the service API will only be called when accessing ranges which have not
    /// been accessed before
    ///
    async fn query(&self, start: Instant, end: Instant) -> Result<Vec<Option<f64>>>;
}
