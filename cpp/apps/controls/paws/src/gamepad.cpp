#include "gamepad.hpp"

#include <algorithm>
#include <cctype>

namespace paws {

namespace {

std::string lowercase(std::string value) {
    std::transform(
        value.begin(),
        value.end(),
        value.begin(),
        [](unsigned char character) {
            return static_cast<char>(std::tolower(character));
        });
    return value;
}

std::optional<size_t> choose_candidate(
    const std::vector<ChoiceKey>& keys,
    const std::vector<size_t>& matching,
    int64_t saved_id) {
    const auto same_id = std::find_if(
        matching.begin(),
        matching.end(),
        [&](size_t index) { return keys[index].id == saved_id; });
    if (same_id != matching.end()) {
        return *same_id;
    }
    if (matching.size() == 1) {
        return matching.front();
    }
    return std::nullopt;
}

}  // namespace

bool is_xbox_style(const std::string& name) {
    const std::string value = lowercase(name);
    return value.find("xbox") != std::string::npos ||
           value.find("xinput") != std::string::npos;
}

std::string label(const std::string& name, int64_t id) {
    return name + " (" + std::to_string(id) + ")";
}

SavedController saved_from_key(const ChoiceKey& key) {
    SavedController saved;
    saved.name = key.name;
    saved.uuid = key.uuid;
    saved.id = key.id;
    return saved;
}

std::optional<size_t> remembered_index(
    const std::vector<ChoiceKey>& keys,
    const SavedController& saved) {
    const bool zero_uuid = std::all_of(
        saved.uuid.begin(),
        saved.uuid.end(),
        [](uint8_t byte) { return byte == 0; });

    std::vector<size_t> matching;
    if (zero_uuid) {
        for (size_t index = 0; index < keys.size(); ++index) {
            if (keys[index].name == saved.name) {
                matching.push_back(index);
            }
        }
    } else {
        for (size_t index = 0; index < keys.size(); ++index) {
            if (keys[index].name == saved.name &&
                keys[index].uuid == saved.uuid) {
                matching.push_back(index);
            }
        }
    }

    if (matching.empty() && zero_uuid) {
        matching.resize(keys.size());
        for (size_t index = 0; index < keys.size(); ++index) {
            matching[index] = index;
        }
    } else if (matching.empty()) {
        for (size_t index = 0; index < keys.size(); ++index) {
            if (keys[index].uuid == saved.uuid) {
                matching.push_back(index);
            }
        }
    }
    return choose_candidate(keys, matching, saved.id);
}

std::optional<int64_t> find_remembered(
    const std::vector<Choice>& choices,
    const SavedController& saved) {
    std::vector<ChoiceKey> keys;
    keys.reserve(choices.size());
    for (const Choice& choice : choices) {
        keys.push_back(choice.key);
    }
    const std::optional<size_t> index = remembered_index(keys, saved);
    if (!index || *index >= choices.size()) {
        return std::nullopt;
    }
    return choices[*index].joystick_id;
}

std::optional<int64_t> selection_for_index(
    int index,
    const std::vector<Choice>& choices) {
    if (index <= 0) {
        return std::nullopt;
    }
    const size_t choice_index = static_cast<size_t>(index - 1);
    if (choice_index >= choices.size()) {
        return std::nullopt;
    }
    return choices[choice_index].joystick_id;
}

int index_for_selection(
    std::optional<int64_t> selected,
    const std::vector<Choice>& choices) {
    if (!selected) {
        return 0;
    }
    for (size_t index = 0; index < choices.size(); ++index) {
        if (choices[index].joystick_id == *selected) {
            return static_cast<int>(index) + 1;
        }
    }
    return 0;
}

}  // namespace paws
