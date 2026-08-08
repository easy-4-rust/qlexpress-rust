/*
{
  "noReturn": true
}
*/
int x = 10;
String result;

switch (x) {
  case 10:
  case 9:
    result = "A";
    break;
  case 8:
    result = "B";
    break;
  default:
    result = "F";
}

assert(result == "A");

// Test multiple cases sharing a code block
function stest(y) {
    switch (y) {
      case 10:
      case 9:
        return "A";
      case 8:
        return "B";
      default:
        return "F";
    }
}

assert(stest(9) == "A");
assert(stest(10) == "A");