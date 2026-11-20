#include "default_format.hpp"

#include <cmath>

namespace display_lib_impl {

namespace {

constexpr float kMargin = 0.0f;
constexpr float kHalfThickness = 0.05f;

using Point = std::array<float, 2>;

Point add(Point a, Point b) {
    return Point{a[0] + b[0], a[1] + b[1]};
}

Point sub(Point a, Point b) {
    return Point{a[0] - b[0], a[1] - b[1]};
}

Point normalize(Point value) {
    const float length = std::sqrt(value[0] * value[0] + value[1] * value[1]);
    if (length == 0.0f) {
        return value;
    }
    return Point{value[0] / length, value[1] / length};
}

void push_bar(std::vector<float>& out, Point a, Point b, std::array<float, 4> color) {
    const Point direction = normalize(sub(b, a));
    const Point normal{
        -direction[1] * kHalfThickness,
        direction[0] * kHalfThickness};
    const Point p0 = add(a, normal);
    const Point p1 = sub(a, normal);
    const Point p2 = add(b, normal);
    const Point p3 = sub(b, normal);
    for (const Point& point : {p0, p1, p2, p2, p1, p3}) {
        out.push_back(point[0]);
        out.push_back(point[1]);
        out.push_back(color[0]);
        out.push_back(color[1]);
        out.push_back(color[2]);
        out.push_back(color[3]);
    }
}

}  // namespace

std::vector<float> default_vertices() {
    std::vector<float> vertices;
    vertices.reserve(12 * 6);
    const float extent = 1.0f - kMargin;
    push_bar(vertices, Point{-extent, -extent}, Point{extent, extent}, kXColor);
    push_bar(vertices, Point{-extent, extent}, Point{extent, -extent}, kXColor);
    return vertices;
}

}  // namespace display_lib_impl
