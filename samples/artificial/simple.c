#include <stdlib.h>
#include <stdio.h>

int main () {
  char* out = malloc(1);
  *out = 'a'; // good
  free(out);
  *out = 'b'; // bad
  printf("past bad! %c\n", *out);
}
