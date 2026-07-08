#include "lib2.h"

extern void world() {
  printf("world !\n");
  static_lib();
  dyn_lib();
}
