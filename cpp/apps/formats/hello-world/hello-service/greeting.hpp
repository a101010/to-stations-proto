#pragma once

#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <string_view>

namespace hello_service {

using Clock = std::chrono::steady_clock;

inline constexpr std::array<std::string_view, 17> kGreetings{
    "Hello, world!",
    "Hola, mundo!",
    "Bonjour le monde!",
    "Hallo, Welt!",
    "Ciao, mondo!",
    "Olá, mundo!",
    "Hallo, wereld!",
    "Hej, världen!",
    "Hei, maailma!",
    "Witaj, świecie!",
    "Merhaba dünya!",
    "Salut, lume!",
    "Salve, munde!",
    "toki, ma o!",
    "Halo, wol!",
    "Habari, dunia!",
    "Pẹlẹ o, ayé!",
};

class GreetingCycle {
public:
    explicit GreetingCycle(Clock::time_point now);
    static GreetingCycle with_seed(Clock::time_point now, uint64_t seed);

    std::string_view current() const;
    bool advance_if_due(Clock::time_point now);
    Clock::time_point next_change() const;

private:
    GreetingCycle(Clock::time_point now, uint64_t seed);

    std::size_t index_ = 0;
    Clock::time_point next_change_;
    uint64_t rng_state_ = 0;
};

}  // namespace hello_service
