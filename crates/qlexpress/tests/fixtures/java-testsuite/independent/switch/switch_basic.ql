int day = 3;
String dayName;

switch (day) {
  case 1:
    dayName = "Monday";
    break;
  case 2:
    dayName = "Tuesday";
    break;
  case 3:
    dayName = "Wednesday";
    break;
  case 4:
    dayName = "Thursday";
    break;
  case 5:
    dayName = "Friday";
    break;
  default:
    dayName = "Weekend";
}

assert(dayName == "Wednesday");

// test default
int num = 10;
String result;
switch (num) {
  case 1:
    result = "one";
    break;
  case 2:
    result = "two";
    break;
  default:
    result = "other";
}

assert(result == "other");