#pragma once

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "state.hpp"

namespace paws {

struct ChoiceKey {
    std::string name;
    std::array<uint8_t, 16> uuid{};
    int64_t id = 0;

    bool operator==(const ChoiceKey&) const = default;
};

struct Choice {
    int64_t joystick_id = 0;
    ChoiceKey key;

    bool operator==(const Choice&) const = default;
};

bool is_xbox_style(const std::string& name);
std::string label(const std::string& name, int64_t id);
SavedController saved_from_key(const ChoiceKey& key);

std::optional<size_t> remembered_index(
    const std::vector<ChoiceKey>& keys,
    const SavedController& saved);
std::optional<int64_t> find_remembered(
    const std::vector<Choice>& choices,
    const SavedController& saved);

std::optional<int64_t> selection_for_index(
    int index,
    const std::vector<Choice>& choices);
int index_for_selection(
    std::optional<int64_t> selected,
    const std::vector<Choice>& choices);

}  // namespace paws
