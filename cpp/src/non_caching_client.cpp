/*
 * @file non_caching_client.cpp
 *
 * This code demonstrates the mechanics (REST + Json) of communicating with the
 * remote server. Combined with the local yaml files, it shows you how to
 * deserialize the responses.
 *
 */
#include <cstdio>
#include <cstdlib>
#include <stdlib.h>
#include <assert.h>
#include <time.h>
#include "restclient-cpp/restclient.h"
#include <string>
#include <iostream>
#include <sstream>
#include "nlohmann/json.hpp"
#include "bandit/bandit.h"
#include "btree/map.h"
#include <vector>
#include <map>
#include <algorithm>
#include <chrono>




using json = nlohmann::json;
using namespace std;
using namespace snowhouse;
using namespace bandit;
using namespace std::chrono;



const int TWO_HOURS = 2 * 60 * 60;
const int ONE_DAY = 24 * 60 * 60;
const int MINUTE = 60;
const int FIVE_MINUTES = 5 * 60;
const int ONE_HOUR = 60 * 60;

typedef std::pair<double, double> key_pair;



class NonCachingClient {
	double lat, lon;

	vector<tuple<int, double>> get_remote_data_five_day_forecast()
	{
	    ostringstream oss;
	    oss << "http://REDACTED"
		<< lat
		<< "&lon="
		<< lon;
	    const string url = oss.str();
	    RestClient::Response r = RestClient::get(url);
	    auto parsed = json::parse(r.body);

	    unsigned response_count = parsed["cnt"]; // number of responses
	    vector<tuple<int, double>> data(response_count);
	    for (auto &element : parsed["list"])
	    {
		data.push_back({element["dt"], element["main"]["temp"]});
	    }
	    return data;
	}

	public:

	NonCachingClient(double lat, double lon) : lat(lat), lon(lon) {};

	vector<double> query(int start, int end) {
		auto data = get_remote_data_five_day_forecast();
		btree::map<int, double> data_map;
		for (auto &tup : data) {
			data_map.insert(tup);
		}
		auto granularity = ONE_HOUR;
		auto requested_range = end - start;
		if (requested_range < TWO_HOURS) {
		    granularity = MINUTE;
		} else if (requested_range < ONE_DAY) {
		    granularity = FIVE_MINUTES;
		};
		vector<double> ret;
		for (int i = start; i < end; i += granularity) {
			auto low = data_map.lower_bound(i);
			if (low == data_map.end()) {
			} else if (low == data_map.begin()) {
				ret.push_back(low->second);
			} else {
				auto prev = std::prev(low);
				if ((i - prev->first) < (low->first - i))
					ret.push_back(prev->second);
				else
					ret.push_back(low->second);
			}
		}
		return ret;
	}
};


/*   Client built with caches
 *
 *   In-memory design was used to build this caching system. The caching design implemented
 *   is a Least Frequently used caching system, which removes the least frequently used 
 *   query when the size of the cache has exceeded. This implementation made use of
 *   three different hash maps, two of which used lan/lon as keys to determine
 *   corresponding data, and frequency. Cache_frequency keeps track of each pair
 *   and how many times its been called. cache_data keeps track of the data corresponding
 *   to the given pair. freq_map holds frequencies as keys and a vector of pairs.
 *   This map is used when removing from the cache. It will pick the LFU which in this case
 *   will always be the first element in the map(given the map is ordered) and find the value
 *   (pair) to be removed from all maps. In event of a tie(multiple pairs in one frequency)
 *   the first element in the vector will be the one removed from the cache as it will be 
 *   the oldest element.Below this map is visualized(as best as possible)
 *
 *			       Oldest pair, and lowest frequency(frequency = 1)
 *						     |
 *						     V
 *		freq_map[frequency = 1] -> <(lat_key1, lon_key1), (lat_key2, lon_key2)>
 *		freq_map[frequency = 2] -> <(lat_key3, lon_key3)
 *				.
 *				.
 *				.
 *		freq_map[frequency = n] -> <(lat_keyK, long_keyK).....>		
 */


class LFU_cache_client {

	unsigned int cache_size;  // cache size == len(hash map)
	double client_lat, client_lon;
	std::map<key_pair, unsigned int> cache_frequency; // map for frequency
	std::map<key_pair, std::vector<tuple<int, double>>> cache_data; // map for data
	std::map<unsigned int, vector<key_pair>> freq_map;	// map containing frequencies and vector of keys for that frequency

        std::vector<tuple<int, double>> get_remote_data_five_day_forecast(){
            ostringstream oss; 
            oss << "http://REDACTED"
                << client_lat
                << "&lon="
                << client_lon;
            const string url = oss.str();
            RestClient::Response r = RestClient::get(url);
            auto parsed = json::parse(r.body);

            unsigned response_count = parsed["cnt"]; // number of responses
            vector<tuple<int, double>> data(response_count);
            for (auto &element : parsed["list"])
            {
                data.push_back({element["dt"], element["main"]["temp"]});
            }
            return data;
        }

	// Deletes a specific key from vector within freq_map
	// done when frequency changes for specific key
        
        void _erase(unsigned int count, key_pair map_key_pair){
                freq_map[count].erase(std::remove(freq_map[count].begin(),
                freq_map[count].end(), map_key_pair), freq_map[count].end());
        }
	
	// Adds a key pair to vector corresponding to its given frequency
	// if frequency doesn't exist, add frequency to map

        void _add(unsigned int count, key_pair map_key_pair){
                if(freq_map.find(count)!=freq_map.end()){
                        freq_map[count].push_back(map_key_pair);
                        return;
                }
                vector<key_pair> temp;
                temp.push_back(map_key_pair);
                freq_map.insert({count, temp});
        }

	// deletes smallest and oldest(if tie) element from all maps
	// if size of map becomes greater than the size it 
	// was init as.

        void _delete(){
                if(freq_map.empty()){
                        return;
                }
                key_pair temp_key = freq_map.begin()->second.front();
                _erase(freq_map.begin()->first, temp_key);
                if(freq_map.begin()->second.empty()){
                        freq_map.erase(freq_map.begin()->first);
                }
                cache_frequency.erase(temp_key);
                cache_data.erase(temp_key);
        }

        // pulls data, checks to see if map is full. If it is,
        // call _delete to remove LFU, if not simply insert into maps(cache)

        vector<tuple<int, double>> _put(){
                key_pair map_key_pair = std::make_pair(client_lat, client_lon);
                auto result = get_remote_data_five_day_forecast();
                if(cache_data.size() >= cache_size){
                        _delete();
                }
                cache_data.insert({map_key_pair, result});
                cache_frequency.insert({map_key_pair, 1});
                _add(cache_frequency[map_key_pair], map_key_pair);
                return result;
        }

	public:

	// Set cache size
	LFU_cache_client(unsigned int cache_size) : cache_size(cache_size) {};
	
	// define lat/lon
	void set_pair(double lat, double lon){
		client_lat = lat; 
		client_lon = lon;
	} 
	
	// looks for pair(lat/lon) in map, if found(hit)
	// add corresponding frequencies to map and return results
	// if not found(cache miss) call _put()

	vector<tuple<int, double>> _get(){
		auto map_key_pair = std::make_pair(client_lat,client_lon);
		if(cache_data.find(map_key_pair)!=cache_data.end()){
			auto result = cache_data[map_key_pair];
			_erase(cache_frequency[map_key_pair], map_key_pair);
			cache_frequency[map_key_pair]++;
			_add(cache_frequency[map_key_pair], map_key_pair);
			return result;
		}
		return _put();
	}
	void _clear(){
		cache_frequency.clear();
		cache_data.clear();
		freq_map.clear();
	}

        vector<double> query(int start, int end) {
                auto data = _get(); // calls _get() instead of restapi method, this allows to check if data is in cache.
                btree::map<int, double> data_map;
                for (auto &tup : data) {
                        data_map.insert(tup);
                }
                auto granularity = ONE_HOUR;
                auto requested_range = end - start;
                if (requested_range < TWO_HOURS) {
                    granularity = MINUTE;
                } else if (requested_range < ONE_DAY) {
                    granularity = FIVE_MINUTES;
                };
                vector<double> ret;
                for (int i = start; i < end; i += granularity) {
                        auto low = data_map.lower_bound(i);
                        if (low == data_map.end()) {
                        } else if (low == data_map.begin()) {
                                ret.push_back(low->second);
                        } else {
                                auto prev = std::prev(low);
                                if ((i - prev->first) < (low->first - i))
                                        ret.push_back(prev->second);
                                else
                                        ret.push_back(low->second);
                        }
                }
                return ret;
        }
	
};




go_bandit([]() {
	const int SAMPLE_DATA_START = 1659722400;
	describe("remote_data", []() {
		it("demonstrates interpolation", [&]() {
			auto start = SAMPLE_DATA_START;
			auto end = start + 25 * ONE_HOUR;
			auto client = NonCachingClient(47.36, -122.19);
			auto data = client.query(start, end);
			AssertThat(data.size(), Equals(25));
			AssertThat(data[0], Equals(290.18));
			AssertThat(data[1], Equals(290.18));
		});

	});
	describe("cache_data", []() {
                it("demonstrates interpolation", [&]() {
                        auto start_cache = SAMPLE_DATA_START;
                        auto end_cache = start_cache + 25 * ONE_HOUR;
			auto cache = LFU_cache_client(10);
                        cache.set_pair(47.36, -122.19);
                        auto data_cache = cache.query(start_cache, end_cache);
                        AssertThat(data_cache.size(), Equals(25));
                        AssertThat(data_cache[0], Equals(290.18));
                        AssertThat(data_cache[1], Equals(290.18));
                });
	});

});

void run_performance_tests(){
	const int SAMPLE_DATA_START = 1659722400;
	
	// Non-Cache Speed Performance 
        auto start = SAMPLE_DATA_START;
        auto end = start + 25 * ONE_HOUR;
        auto client = NonCachingClient(47.36, -122.19);
        auto data = client.query(start, end);
	auto start_time = high_resolution_clock::now();

	for(int i = 0; i < 3000; i++){
		client = NonCachingClient(47.36, -122.19);
		data = client.query(start, end);
	}
	auto stop = high_resolution_clock::now();
	auto duration = duration_cast<microseconds>(stop - start_time);
        cout << "________________________________________" << endl;
	cout << "Non-Cache Client performance on 3000 queries: " <<  duration.count() << " microseconds" << endl;
	auto temp = duration;

	// Cache Speed Performance
        auto start_cache = SAMPLE_DATA_START;
        auto end_cache = start_cache + 25 * ONE_HOUR;
        auto cache = LFU_cache_client(10);
        cache.set_pair(47.36, -122.19);
        auto data_cache = cache.query(start_cache, end_cache);

        start_time = high_resolution_clock::now();
	for(int i = 0; i < 3000; i++){
		cache.set_pair(47.36, -122.19);
		data_cache = cache.query(start_cache, end_cache);
	}
        stop = high_resolution_clock::now();
        duration = duration_cast<microseconds>(stop - start_time);
        cout << "Cache Client performance on 3000 queries: " <<  duration.count() << " microseconds" << endl;


	cout << "Cache Performs roughly "<<  temp/duration << " times faster than the non-cache client!" << endl;
        cout << "________________________________________" << endl;

}

int main(int argc, char *argv[])
{
	run_performance_tests();
	return bandit::run(argc, argv);
}
