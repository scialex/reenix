#include "kernel.h"
#include "types.h"
#include "util/debug.h"
#include "drivers/bytedev.h"
#include "util/list.h"
#include "drivers/tty/tty.h"
#include "drivers/memdevs.h"

static list_t bytedevs;

void
bytedev_init()
{
        list_init(&bytedevs);
        /* Initialize all subsystems */
        tty_init();
        memdevs_init();
}

int
bytedev_register(bytedev_t *dev)
{
        bytedev_t *cd;

        /* Make sure dev, dev ops, and dev id not null */
        if (!dev
            || (NULL_DEVID == dev->cd_id)
            || !(dev->cd_ops))
                return -1;

        /* we should not have already seen dev->cd_id. */
        list_iterate_begin(&bytedevs, cd, bytedev_t, cd_link) {
                if (dev->cd_id == cd->cd_id)
                        return -1;
        } list_iterate_end();

        /* initialize portions of structure ignored by device drivers: */

        list_insert_tail(&bytedevs, &dev->cd_link);
        return 0;
}

bytedev_t *
bytedev_lookup(devid_t id)
{
        bytedev_t *cd;

        list_iterate_begin(&bytedevs, cd, bytedev_t, cd_link) {
                KASSERT(NULL_DEVID != cd->cd_id);
                if (id == cd->cd_id)
                        return cd;
        } list_iterate_end();

        return NULL;
}
