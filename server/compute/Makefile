CXX ?= g++
CXXFLAGS ?= -std=c++17 -O2 -Wall -Wextra -pedantic

all: compute_module

compute_module: compute.cpp
	$(CXX) $(CXXFLAGS) compute.cpp -o compute_module

clean:
	rm -f compute_module
