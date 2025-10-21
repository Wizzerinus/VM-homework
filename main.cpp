#include <algorithm>
#include <atomic>
#include <cassert>
#include <chrono>
#include <ctime>
#include <iostream>
#include <new>
#include <thread>
#include <vector>

using ATOMIC_T = std::atomic_int64_t;

constexpr size_t ALIGNMENT = 1 << 12;
unsigned char *buffer = nullptr;
unsigned char *eater = nullptr;
size_t BUFFER_SIZE = 0;
const constexpr size_t EATER_SIZE = 1 << 26;
const constexpr size_t FALSE_SHARING_TESTLEN = 1 << 26;
ATOMIC_T *atomics = nullptr;
unsigned char eaten_count = 0;

void make_buffer(size_t size) {
  if (buffer != nullptr) {
    delete[] buffer;
  }

  buffer =
      new (std::align_val_t(ALIGNMENT)) unsigned char[size * sizeof(void *)];
  BUFFER_SIZE = size;
}

void make_eater() {
  if (eater != nullptr) {
    delete[] eater;
  }

  eater = new (std::align_val_t(ALIGNMENT)) unsigned char[EATER_SIZE];
  for (size_t i = 0; i < EATER_SIZE; i++) {
    eater[i] = i;
  }
}

void make_atomics(size_t count) {
  if (atomics != nullptr) {
    delete[] atomics;
  }

  atomics = new (std::align_val_t(ALIGNMENT)) ATOMIC_T[count];
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
    assert(eater[i] == static_cast<unsigned char>(i + eaten_count));
  }
  for (size_t i = 0; i < EATER_SIZE; i++) {
    eater[i]++;
  }
  eaten_count++;
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

void do_false_sharing(size_t guess) {
  for (size_t count = 0; count != FALSE_SHARING_TESTLEN; count++)
    atomics[guess].fetch_add(1, std::memory_order_relaxed);
}

void test_false_sharing(size_t guess) {
  std::thread fst([]() { return do_false_sharing(0); });
  std::thread snd([guess]() { return do_false_sharing(guess); });
  fst.join();
  snd.join();
}

size_t compute_linelength() {
  double last_time = -1;
  for (size_t guess = 8; guess <= 1024; guess <<= 1) {
    auto time = std::chrono::steady_clock::now();
    test_false_sharing(guess / sizeof(ATOMIC_T));
    auto time2 = std::chrono::steady_clock::now();
    std::chrono::duration<double> diff = time2 - time;

    double dur = diff.count();
    cout << "Guess: " << guess << " time: " << dur << "\n";
    if (last_time > 0 && dur * 2 < last_time) {
      cout << "Likely line length: " << guess << "\n\n";
      return guess;
    }
    last_time = dur;
  }

  return 0;
}

bool reasonable_guess(size_t guess) {
  while (guess % 2 == 0)
    guess >>= 1;
  return guess < 6;
}

struct Guess {
  double fraction;
  size_t guess;
};

size_t compute_line_count(size_t linelength) {
  linelength /= sizeof(void *);
  double last_time = -1;
  size_t prev_guess = 0;
  vector<Guess> guesses;
  guesses.push_back({1.0, 0});
  for (size_t guess = 128; guess <= 1024; guess += 32) {
    if (!reasonable_guess(guess))
      continue;
    vector<size_t> pattern(guess - 1);
    for (size_t i = 0; i < guess - 1; i++) {
      pattern[i] = (i + 1) * linelength;
    }

    double time = compute_time(BUFFER_SIZE, pattern);
    cout << "Guess: " << guess << " time: " << time << std::endl;

    double frac = time / last_time;
    if (frac > 0)
      guesses.push_back({frac, prev_guess});
    last_time = time;
    prev_guess = guess;
  }

  std::sort(guesses.begin(), guesses.end(), [](const Guess &l, const Guess &r) {
    return r.fraction < l.fraction;
  });
  double best_fraction = guesses[0].fraction,
         frac_delta = guesses[0].fraction - guesses[1].fraction;

  if (best_fraction > 1.09 && frac_delta > 0.04) {
    cout << "Likely line count: " << guesses[0].guess
         << " (confidence fraction: " << best_fraction << ", gap " << frac_delta
         << ")" << "\n\n";
    return guesses[0].guess;
  }

  cout << "Unable to determine line count with good confidence (best fraction: "
       << best_fraction << ", gap " << frac_delta << "), retrying...\n\n";
  return 0;
}

size_t compute_associativity(size_t linelength, size_t line_count) {
  double last_time = -1;
  for (size_t guess = 1; guess <= line_count; guess++) {
    if (line_count % guess != 0)
      continue; // Associativity must be uniform

    size_t pattern_size = guess; // start=0 is not counted
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
  make_atomics(1 << 10);
  cout << "Guessing line length; expecting major time decrease on correct guess"
       << std::endl;
  size_t linelength = compute_linelength();
  if (!linelength) {
    cout << "Unable to predict linelength";
    return 1;
  }

  cout << "Guessing line count; expecting minor time increase after correct "
          "guess"
       << std::endl;

  size_t line_count = 0;
  for (size_t i = 0; i < 5 && line_count == 0; i++) {
    line_count = compute_line_count(linelength);
    if (!line_count)
      eat_cache();
  }
  if (!line_count) {
    cout << "Unable to predict line count after 5 tries";
    return 1;
  }

  cout << "Guessing associativity; expecting major time increase on correct "
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
  delete[] atomics;
}