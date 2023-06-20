//! Simple command line driver for the caching API code
use clap::{ArgEnum, Parser};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the city
    #[clap(arg_enum, value_parser)]
    city: XetCity,

    /// Time duration for forecast
    #[clap(short, long, value_parser)]
    duration: humantime::Duration,
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum XetCity {
    Seattle,   // XetData is built with love in the Emerald City
    Vancouver, // and Vancouver, WA!
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let (_lat, _long) = match args.city {
        XetCity::Seattle => (47.36, -122.19),
        XetCity::Vancouver => (45.62, -122.67),
    };

    println!("Forecasted temperature is below:");
}
