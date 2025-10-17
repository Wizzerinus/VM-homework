#include <algorithm>
#include <chrono>
#include <ctime>
#include <iostream>
#include <new>
#include <random>

constexpr size_t ALIGNMENT = 1 << 12;
size_t STEPS = 1 << 26;
unsigned volatile char *buffer = nullptr, *eater = nullptr;
volatile size_t *jumps = nullptr;
size_t JUMP_SIZE = 0;

template <typename T> void make_jumps(T &rng, size_t stride) {
  if (jumps != nullptr)
    delete[] jumps;
  size_t count = JUMP_SIZE = STEPS / stride;
  jumps = new size_t[count];
  for (int i = 0; i < count; i++) {
    jumps[i] = i * stride;
  }
  std::shuffle(jumps, jumps + count, rng);
}

template <typename T> void make_buffers(T &rng) {
  if (jumps != nullptr) {
    delete[] buffer;
    delete[] eater;
  }
  buffer = new (std::align_val_t(ALIGNMENT)) unsigned char[STEPS];
  eater = new (std::align_val_t(ALIGNMENT)) unsigned char[STEPS];
  for (size_t i = 0; i < STEPS; i++) {
    buffer[i] = rng();
    eater[i] = rng();
  }
}

using std::cout, std::endl;
using std::mt19937;

void eat_cache() {
  for (size_t i = 0; i < STEPS; i++) {
    eater[i];
  }
}

template <typename F, typename... Ts>
size_t compute_time(const F &callback, const Ts &...params) {
  eat_cache();
  auto time = std::chrono::steady_clock::now();
  callback(params...);
  auto time2 = std::chrono::steady_clock::now();
  return (time2 - time).count();
}

void linelength_subroutine(size_t exp_linelength, size_t stride) {
  for (int k = 0; k < stride; k++) {
    for (size_t i = 0; i < STEPS; i += exp_linelength) {
      size_t x = jumps[i];
      for (size_t j = k; j < exp_linelength; j += stride) {
        buffer[x + j];
      }
    }
  }
}

template <typename T> size_t guess_linelength(T &rng) {
  /*
    Сравниваем время прохода линейки с шагом 2 дважды с временем прохода линейки
    с шагом 1 единожды

    Теория (после написания алгоритма):
    Если длина линейки == i, то время должно отличаться мало - в 1 случае мы
    проходим по всей линейне, а в 2 случае 2 раза её загружаем Если длина
    линейки < i, то теряем много времени на переключение линеек, но мб и повезёт
    Если длина линейки > i, то нужно при каждом проходе стягивать несколько
    линеек, видимо тут проблема

    Теория (реальная):
    А фиг её знает

    На -O0 не работает(
  */

  make_jumps(rng, 1);
  size_t best_fit = 0;
  double best_ratio = 0.1;
  for (size_t i = 16; i <= 1024; i <<= 1) {
    size_t fst = compute_time(linelength_subroutine, i, 1),
           snd = compute_time(linelength_subroutine, i, 2);
    double ratio = static_cast<double>(fst) / snd;
    cout << "Length: " << i << ", ratio: " << ratio << "\n";
    if (ratio > best_ratio) {
      best_fit = i;
      best_ratio = ratio;
    }
  }
  return best_fit;
}

void cachesize_subroutine(size_t guess, size_t linelength) {
  // JUMP_SIZE ~ 1 / guess, 8 * linelength ~ 1, (target - start) ~ guess
  for (size_t i = 0; i <= JUMP_SIZE; i++) {
    for (size_t s = 0; s < 8 * linelength; s++) {
      size_t start = jumps[i] * (s % 2);
      size_t target = start + guess;
      for (size_t j = start; j < target; j += linelength) {
        buffer[j];
      }
    }
  }
}

template <typename T> size_t guess_cachesize(T &rng, size_t linelength) {
  /*
    Пытаемся много раз пройти по кешу

    Если cachesize >= i, то весь проход влезет в кеш, если cachesize < i, то
    будут постоянные вытеснения
  */

  size_t best_fit = 0;
  double best_ratio = 2.0;
  for (size_t i = 1024; i <= 1048576; i <<= 1) {
    make_jumps(rng, i);
    size_t prev = compute_time(cachesize_subroutine, i, linelength);
    make_jumps(rng, i >> 1);
    size_t cur = compute_time(cachesize_subroutine, i >> 1, linelength);
    double ratio = static_cast<double>(cur) / prev;
    cout << "Size: " << i << ", ratio: " << ratio << "\n";
    if (ratio < best_ratio) {
      best_fit = i;
      best_ratio = ratio;
    }
  }
  return best_fit;
}

void assoc_subroutine(size_t guess, size_t cache_size) {
  size_t loop_cnt = 5000;
  for (int i = 0; i < JUMP_SIZE; i += guess) {
    for (int k = 0; k < loop_cnt; k++) {
      for (int j = 0; j < guess; j++) {
        buffer[jumps[j]];
      }
    }
  }
}

template <typename T>
size_t guess_assoc(T &rng, size_t linelength, size_t cache_size) {
  /*
  Пытаемся пройти по set_count+1 точкам с stride = cache_size

  Если мы угадали set_count, то будут вытеснения, а на предыдущем значении guess
  не будет вытеснений
  */

  make_jumps(rng, cache_size);
  size_t line_count = cache_size / linelength;

  size_t best_fit = 0;
  // assoc=1 is not possible on modern machines and gives false positives
  for (size_t i = 2; i < line_count; i++) {
    if (line_count % i != 0)
      // non-uniform associativity should be invalid
      continue;
    size_t cur = compute_time(assoc_subroutine, line_count / i, cache_size);
    size_t next =
        compute_time(assoc_subroutine, line_count / i / 2, cache_size);
    double ratio = static_cast<double>(cur) / next;
    cout << "Assoc: " << i << ", ratio: " << ratio << "\n";
    if (ratio > 1.15) { // тут константа процессорнозависимая :(
      best_fit = i;
      break;
    }
  }

  return best_fit;
}

int main() {
  cout << "Generating buffer..." << endl;
  mt19937 rng(time(nullptr));
  make_buffers(rng);
  cout << "Starting computation..." << endl;

  cout << "Getting linelength..." << endl;
  int linelength = guess_linelength(rng);
  cout << "\nGetting cache size..." << endl;
  int cache_size = guess_cachesize(rng, linelength);
  cout << "\nScaling buffers..." << endl;
  STEPS = 1 << 30;
  make_buffers(rng);
  cout << "\nGetting assoc count..." << endl;
  int assoc_count = guess_assoc(rng, linelength, cache_size);

  cout << "\nLine length: " << linelength << "\nCache size:  " << cache_size
       << "\nAssoc count: " << assoc_count << endl;
  delete[] buffer;
  delete[] eater;
  delete[] jumps;
}
