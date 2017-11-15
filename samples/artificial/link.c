#include <stdlib.h>
char* h(void);
void j(char*);
void i(char*);
int main () {
  char* ptr = h();
  *ptr = 'a';
  j(ptr);
  i(ptr);
  *ptr = 'b';
  j(ptr);
}
