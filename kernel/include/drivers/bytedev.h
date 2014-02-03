#pragma once

#include "drivers/dev.h"
#include "util/list.h"

struct vnode;
struct pframe;

struct bytedev_ops;
struct vmarea;
struct mmobj;

typedef struct bytedev {
        devid_t             cd_id;
        struct bytedev_ops *cd_ops;
        list_link_t         cd_link;
} bytedev_t;

typedef struct bytedev_ops {
        int (*read)(bytedev_t *dev, int offset, void *buf, int count);
        int (*write)(bytedev_t *dev, int offset, const void *buf, int count);
        int (*mmap)(struct vnode *file, struct vmarea *vma, struct mmobj **ret);
        int (*fillpage)(struct vnode *file, off_t offset, void *pagebuf);
        int (*dirtypage)(struct vnode *file, off_t offset);
        int (*cleanpage)(struct vnode *file, off_t offset, void *pagebuf);
} bytedev_ops_t;

/**
 * Initializes the byte device subsystem.
 */
void bytedev_init(void);

/**
 * Registers the given byte device.
 *
 * @param dev the byte device to register
 */
int bytedev_register(bytedev_t *dev);

/**
 * Finds a byte device with a given device id.
 *
 * @param id the device id of the byte device to find
 * @return the byte device with the given id if it exists, or NULL if
 * it cannot be found
 */
bytedev_t *bytedev_lookup(devid_t id);
