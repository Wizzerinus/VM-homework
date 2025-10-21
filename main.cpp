#include <cassert>
#include <chrono>
#include <ctime>
#include <iostream>
#include <new>
#include <vector>

constexpr size_t ALIGNMENT = 1 << 12;
unsigned char *buffer = nullptr;
unsigned char *eater = nullptr;
size_t BUFFER_SIZE = 0;
const constexpr size_t EATER_SIZE = 1 << 26;

void make_buffer(size_t size) {
  if (buffer != nullptr) {
    delete[] buffer;
  }

  buffer =
      new (std::align_val_t(ALIGNMENT)) unsigned char[size * sizeof(void *)];
  BUFFER_SIZE = size;
}

void make_eater() {
  eater = new (std::align_val_t(ALIGNMENT)) unsigned char[EATER_SIZE];
  for (size_t i = 0; i < EATER_SIZE; i++) {
    eater[i] = i;
  }
}

using std::cout, std::vector;

void make_pattern(const std::vector<size_t> &pattern) {
  size_t pos = 0;
  constexpr size_t ptr_size = sizeof(void *);

  // Make sure the pattern does not terminate too early.
  for (auto it : pattern) {
    *reinterpret_cast<void **>(buffer + pos * ptr_size) = nullptr;
    pos = it;
  }

  // Generate the pattern.
  pos = 0;
  for (auto it : pattern) {
    auto ptr = reinterpret_cast<void **>(buffer + pos * ptr_size);
    assert(*ptr == nullptr);
    *ptr = reinterpret_cast<void *>(buffer + it * ptr_size);
    pos = it;
  }
  // pattern ->> cyclic
  *reinterpret_cast<void **>(buffer + pos * ptr_size) =
      reinterpret_cast<void *>(buffer);
}

void cycle(size_t cycles) {
  void **pointer = reinterpret_cast<void **>(buffer);
  while (cycles--) {
    pointer = static_cast<void **>(*pointer);
  }
  // try to guarantee that a side effect happens
  assert(pointer != nullptr);
}

void eat_cache() {
  for (size_t i = 0; i < EATER_SIZE; i++) {
    assert(eater[i] == static_cast<unsigned char>(i));
  }
}

double compute_time(size_t cycles, const std::vector<size_t> &pattern) {
  make_pattern(pattern);
  void **pointer;

  // Warmup

  cycle(cycles);
  cycle(cycles);
  eat_cache();

  // Perform the measurement
  auto time = std::chrono::steady_clock::now();
  cycle(cycles);
  auto time2 = std::chrono::steady_clock::now();

  std::chrono::duration<double> diff = time2 - time;
  return diff.count();
}

size_t compute_linelength() {
  size_t MAX_CACHE_LINES = 1 << 16;
  vector<size_t> pattern(MAX_CACHE_LINES);

  double last_time = -1;
  for (size_t guess = 16; guess <= 1024; guess <<= 1) {
    for (size_t i = 0; i < MAX_CACHE_LINES; i++) {
      pattern[i] = (i + 1) * (guess * 3 / 2) / sizeof(void *);
    }

    double time = compute_time(BUFFER_SIZE, pattern);
    cout << "Guess: " << guess << " time: " << time << std::endl;
    if (last_time > 0 && last_time * 1.2 < time) {
      cout << "Likely line length: " << guess << "\n\n";
      return guess;
    }
    last_time = time;
  }

  return 0;
}

size_t compute_line_count(size_t linelength) {
  linelength /= sizeof(void *);
  double last_time = -1;
  for (size_t guess = 16; guess <= 4096; guess <<= 1) {
    vector<size_t> pattern(guess);
    for (size_t i = 0; i < guess; i++) {
      // see above
      size_t j = i % 2 == 0 ? 0 : (i % 4 == 1 ? 2 : -2);
      pattern[i] = (i + 1 + j) * linelength;
    }

    double time = compute_time(BUFFER_SIZE, pattern);
    cout << "Guess: " << guess << " time: " << time << std::endl;

    if (last_time > 0 && last_time * 1.15 < time) {
      cout << "Likely line count: " << guess / 2 << "\n\n";
      return guess / 2;
    }
    last_time = time;
  }

  return 0;
}

size_t compute_associativity(size_t linelength, size_t line_count) {
  double last_time = -1;
  for (size_t guess = 1; guess <= line_count; guess++) {
    if (line_count % guess != 0)
      continue; // Associativity must be uniform

    size_t pattern_size = guess + 1;
    // If associativity = guess, then there are =guess different values, that
    // map to the same point. Since we try to map guess + 1 values, we should
    // encounter thrashing on every read.
    // If associativity > guess, then we should not encounter any thrashing.
    // Therefore, at the correct guess, we should get a major performance loss.
    vector<size_t> pattern(pattern_size);
    // skipping cache_size bytes will definitely be in the same associative set
    size_t cache_size_ptrs = linelength * line_count / sizeof(void *);
    for (size_t i = 0; i < pattern_size; i++) {
      pattern[i] = (i + 1) * cache_size_ptrs;
    }
    double time = compute_time(BUFFER_SIZE, pattern);
    cout << "Guess: " << guess << " time: " << time << std::endl;
    // Some CPUs have much weaker thrashing effects for some reason.
    if (last_time > 0 && last_time * 1.2 < time) {
      cout << "Likely associativity: " << guess << "\n\n";
      return guess;
    }
    last_time = time;
  }

  return 1;
}

int main() {
  make_buffer(1 << 28);
  make_eater();
  cout << "Guessing line length; expecting minor time increase on correct guess"
       << std::endl;
  size_t linelength = compute_linelength();
  if (!linelength) {
    cout << "Unable to predict linelength";
    return 1;
  }

  cout << "Guessing line count; expecting minor time increase AFTER correct "
          "guess"
       << std::endl;
  size_t line_count = compute_line_count(linelength);
  if (!line_count) {
    cout << "Unable to predict line count";
    return 1;
  }

  cout << "Guessing associativity; expecting MAJOR time increase on correct "
          "guess"
       << std::endl;
  size_t associativity = compute_associativity(linelength, line_count);
  if (!associativity) {
    cout << "Unable to predict associativity";
    return 1;
  }

  cout << "Predicted line length: " << linelength << "\n";
  cout << "Predicted line count: " << line_count
       << " (cache size: " << linelength * line_count << ")\n";
  cout << "Predicted " << associativity << "-way associativity ("
       << line_count / associativity << " lines per set)" << "\n";

  delete[] buffer;
  delete[] eater;
}