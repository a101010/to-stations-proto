#include <cmath>
#include <iostream>
#include <vector>

#include "default_format.hpp"

namespace {

int failures = 0;

void check(bool condition, const char* message) {
    if (!condition) {
        std::cerr << "FAIL: " << message << "\n";
        ++failures;
    }
}

}  // namespace

int main() {
    const std::vector<float> vertices = display_lib_impl::default_vertices();
    check(vertices.size() == 12 * 6, "12 vertices");

    constexpr float limit = 1.0f + 0.05f;
    for (size_t offset = 0; offset + 5 < vertices.size(); offset += 6) {
        check(std::abs(vertices[offset]) <= limit, "x in range");
        check(std::abs(vertices[offset + 1]) <= limit, "y in range");
    }

    check(
        display_lib_impl::kXColor[0] < 0.1f &&
            display_lib_impl::kXColor[1] > 0.4f &&
            display_lib_impl::kXColor[2] < 0.2f,
        "green x color");

    if (failures != 0) {
        std::cerr << failures << " test(s) failed\n";
        return 1;
    }
    std::cout << "display-lib tests passed\n";
    return 0;
}
