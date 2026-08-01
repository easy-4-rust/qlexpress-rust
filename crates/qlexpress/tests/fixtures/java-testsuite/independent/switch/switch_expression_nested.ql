// Nested switch expressions
x = 1
y = 2

result = switch (x) {
    case 1 -> switch (y) {
        case 1 -> "1-1"
        case 2 -> "1-2"
        default -> "1-other"
    }
    case 2 -> "2"
    default -> "other"
}

assert(result == "1-2")
