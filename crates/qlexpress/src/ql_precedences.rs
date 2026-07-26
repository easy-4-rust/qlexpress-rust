//! Operator precedence constants, mirroring Java `QLPrecedences`.

/// `= += -= &= |= *= /= %= <<= >>=`
pub const ASSIGN: i32 = 0;

/// `?:`
pub const TERNARY: i32 = 1;

/// `|| or`
pub const OR: i32 = 2;

/// `&& and`
pub const AND: i32 = 3;

/// `|`
pub const BIT_OR: i32 = 4;

/// `^`
pub const XOR: i32 = 5;

/// `&`
pub const BIT_AND: i32 = 6;

/// `== !=`
pub const EQUAL: i32 = 7;

/// `< <= > >= instanceof`
pub const COMPARE: i32 = 8;

/// `<< >> >>>`
pub const BIT_MOVE: i32 = 9;

/// `in like`
pub const IN_LIKE: i32 = 10;

/// `+ -`
pub const ADD: i32 = 11;

/// `* / %`
pub const MULTI: i32 = 12;

/// `! ++ -- ~ + -`
pub const UNARY: i32 = 13;

/// `++ --` in suffix position, like `i++`
pub const UNARY_SUFFIX: i32 = 14;

/// `. ()`
pub const GROUP: i32 = 15;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_order_is_strictly_increasing() {
        let ordered = [
            ASSIGN,
            TERNARY,
            OR,
            AND,
            BIT_OR,
            XOR,
            BIT_AND,
            EQUAL,
            COMPARE,
            BIT_MOVE,
            IN_LIKE,
            ADD,
            MULTI,
            UNARY,
            UNARY_SUFFIX,
            GROUP,
        ];
        for (i, window) in ordered.windows(2).enumerate() {
            assert!(window[0] < window[1], "precedence broke at index {i}");
        }
        assert_eq!(ASSIGN, 0);
        assert_eq!(GROUP, 15);
    }
}
