#include "support/single_instance.hpp"

#include <string>
#include <utility>

#include <windows.h>

namespace support {

struct SingleInstance::Impl {
    HANDLE handle = nullptr;

    ~Impl() {
        if (handle != nullptr) {
            CloseHandle(handle);
        }
    }
};

namespace {

std::wstring to_wide(std::string_view value) {
    if (value.empty()) {
        return {};
    }
    const int length = MultiByteToWideChar(
        CP_UTF8,
        0,
        value.data(),
        static_cast<int>(value.size()),
        nullptr,
        0);
    if (length <= 0) {
        return {};
    }
    std::wstring result(static_cast<size_t>(length), L'\0');
    MultiByteToWideChar(
        CP_UTF8,
        0,
        value.data(),
        static_cast<int>(value.size()),
        result.data(),
        length);
    return result;
}

}  // namespace

SingleInstance::SingleInstance(std::unique_ptr<Impl> impl)
    : impl_(std::move(impl)) {}

SingleInstance::SingleInstance(SingleInstance&&) noexcept = default;

SingleInstance& SingleInstance::operator=(SingleInstance&&) noexcept = default;

SingleInstance::~SingleInstance() = default;

std::optional<SingleInstance> SingleInstance::acquire(std::string_view tool_name) {
    const std::wstring name = L"Local\\to-stations-" + to_wide(tool_name);
    SetLastError(0);
    HANDLE handle = CreateMutexW(nullptr, FALSE, name.c_str());
    if (handle == nullptr) {
        return std::nullopt;
    }
    if (GetLastError() == ERROR_ALREADY_EXISTS) {
        CloseHandle(handle);
        return std::nullopt;
    }
    auto impl = std::make_unique<Impl>();
    impl->handle = handle;
    return SingleInstance(std::move(impl));
}

}  // namespace support
