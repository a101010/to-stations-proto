#include "greeting.hpp"

#include <chrono>

namespace hello_service {

namespace {

constexpr std::chrono::seconds kMinChange{15};
constexpr std::chrono::seconds kMaxChange{60};

uint64_t next_random(uint64_t& state) {
    state += 0x9e3779b97f4a7c15ULL;
    uint64_t z = state;
    z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL;
    z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL;
    return z ^ (z >> 31);
}

std::chrono::seconds random_interval(uint64_t& state) {
    constexpr uint64_t span = 46;
    return kMinChange + std::chrono::seconds{next_random(state) % span};
}

uint64_t initial_seed() {
    const auto now = std::chrono::system_clock::now().time_since_epoch();
    const uint64_t count = static_cast<uint64_t>(
        std::chrono::duration_cast<std::chrono::nanoseconds>(now).count());
    return count == 0 ? 0x9e3779b97f4a7c15ULL : count;
}

}  // namespace

GreetingCycle::GreetingCycle(Clock::time_point now)
    : GreetingCycle(now, initial_seed()) {}

GreetingCycle GreetingCycle::with_seed(Clock::time_point now, uint64_t seed) {
    return GreetingCycle(now, seed);
}

GreetingCycle::GreetingCycle(Clock::time_point now, uint64_t seed)
    : rng_state_(seed) {
    next_change_ = now + random_interval(rng_state_);
}

std::string_view GreetingCycle::current() const {
    return kGreetings[index_];
}

bool GreetingCycle::advance_if_due(Clock::time_point now) {
    bool changed = false;
    while (now >= next_change_) {
        index_ = (index_ + 1) % kGreetings.size();
        next_change_ += random_interval(rng_state_);
        changed = true;
    }
    return changed;
}

Clock::time_point GreetingCycle::next_change() const {
    return next_change_;
}

}  // namespace hello_service
