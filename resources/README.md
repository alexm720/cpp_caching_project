# Running a mock server
## httpmock
[httpmock](https://github.com/alexliesenfeld/httpmock) is a mock HTTP server, check
it out and build it in the following manner:
```
git clone https://github.com/alexliesenfeld/httpmock.git
cargo build --features=standalone
```
## Running httpmock with the mock yaml files
```
./target/debug/httpmock -e -p 50000 -m <dir>/resources/
```
These mock yaml files are queried periodically from the [openweather
API](https://openweathermap.org/api/one-call-3)
## Accessing the mock server via curl
- seattle.yaml
```
curl "http://localhost:50000/data/2.5/forecast?lat=47.36&lon=-122.19"
```
- vancouver.yaml
This is the output of `curl "https://api.openweathermap.org/data/2.5/forecast?lat=45.62&lon=-122.67&appid=seekrit"`
```
curl "http://localhost:50000/data/2.5/forecast?lat=45.62&lon=-122.67"
```
