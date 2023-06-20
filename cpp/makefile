SRC=./src
TARGET=.
#  Flags will need to be modified based on the install locations
CC = clang++ -std=c++17 -stdlib=libc++
CFLAGS = -o $(TARGET)/non_caching_client.out -I/usr/local/Cellar/libcurl/include/ \
	 -I/usr/local/include -I/Users/ajitb/oss/2022h2/cpp/cpp-btree \
	 -I/Users/alexmedina/oss/cpp/restclient-cpp \ 
	 -I/Users/ajitb/oss/2022h2/cpp/bandit \
	 -L/opt/homebrew/Cellar/curl/7.84.0/lib/ -lcurl -L/usr/local/lib \
	 -lrestclient-cpp
RM = rm -rf

all: clean default test

default: non_caching_client.cpp

non_caching_client.cpp:
	$(CC) $(CFLAGS) $(SRC)/non_caching_client.cpp

test: non_caching_client.cpp
	$(TARGET)/non_caching_client.out

clean:
	$(RM) $(TARGET)/*.out
