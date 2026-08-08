// nested switch
int x = 1;
int y = 2;
String result;

switch (x) {
  case 1:
    switch (y) {
      case 1:
        result = "1-1";
        break;
      case 2:
        result = "1-2";
        break;
      default:
        result = "1-other";
    }
    break;
  case 2:
    result = "two";
    break;
  default:
    result = "other";
}

assert(result == "1-2");

// switch nested in if
int a = 5;
String msg;

if (a > 0) {
  switch (a) {
    case 1:
      msg = "one";
      break;
    case 5:
      msg = "five";
      break;
    default:
      msg = "other";
  }
} else {
  msg = "negative";
}

assert(msg == "five");
