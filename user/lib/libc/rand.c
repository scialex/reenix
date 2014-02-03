#include <stdlib.h>

/* Random int between lo and hi inclusive */

/* TODO Fix the rand/srand implementation to use the implementation in the
 * (unused) macro which actually has decent pseudo-randomness properties. (No,
 * you can't just change the mod base and expect it to still work fine...) */

#define RANDOM(lo,hi) ((lo)+(((hi)-(lo)+1)*(randseed = (randseed*4096+150889)%714025))/714025)

static unsigned long long randseed = 123456L;

int rand(void)
{
        randseed = (randseed * 4096 + 150889) % RAND_MAX;
        return randseed;
}

void srand(unsigned int seed)
{
        randseed = seed;
}
