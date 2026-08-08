// tag::nestedSpreadExample[]
// Use case 1: Spread nested list to get field values
l = [[{"a":10},{"a":12}],[{"a":13}],[{"a":14}]]
result = l*.a
assert(result == [10, 12, 13, 14])

// Use case 2: Spread nested list to call methods
l2 = [[{"a":10},{"a":12}],[{"a":13}],[{"a":14}]]
result2 = l2*.get("a")
assert(result2 == [10, 12, 13, 14])

// Use case 3: if field exists at current level, don't flatten
l3 = [[{"a":10},{"a":12}],[{"a":13}],[{"a":14}]]
result3 = l3*.length
assert(result3 == [2, 1, 1])
// end::nestedSpreadExample[]

// Additional test: nested arrays
arr = [[1, 2], [3, 4, 5]]
// Arrays should also support nested spread
// For now, let's test if arrays have length property
arr_result = arr*.length
assert(arr_result == [2, 3])

// Test with deeper nesting (3 levels)
l4 = [[[{"a":1}], [{"a":2}]], [[{"a":3}]]]
result4 = l4*.a
assert(result4 == [1, 2, 3])
