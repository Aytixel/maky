#include <deps/lib/lib.h>
#include <deps/helloworld/lib.h>

//@import lib/test, helloworld/helloworld
int main()
{
	hello();
	world();
	helloworld();

	return 0;
}