// Basic switch expression with arrow syntax
// tag::switchExpression[]
score = 70
result = switch (score) {
    case 90, 100 -> "优秀"
    case 60, 70, 80 -> "及格"
    default -> "不及格"
}
assert(result == "及格")
// end::switchExpression[]

// Test with numbers
num = 2
result2 = switch (num) {
    case 1 -> 10
    case 2 -> 20
    case 3 -> 30
    default -> 0
}
assert(result2 == 20)

// Test with default
num3 = 999
result3 = switch (num3) {
    case 1, 2, 3 -> "small"
    default -> "large"
}
assert(result3 == "large")

// Test single case value
status = 1
result4 = switch (status) {
    case 0 -> "off"
    case 1 -> "on"
    default -> "unknown"
}
assert(result4 == "on")
