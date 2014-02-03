/*
 *       FILE: dev_byte.h
 *      DESCR: device management: block-oriented devices
 */

#pragma once

#include "types.h"

#include "drivers/dev.h"
#include "util/list.h"

#include "mm/page.h"
#include "mm/mmobj.h"

#define BLOCK_SIZE PAGE_SIZE

struct blockdev_ops;

/*
 * Represents a Weenix block device.
 */
typedef struct blockdev {
        /* Fields that should be initialized by drivers: */
        devid_t bd_id;

        struct blockdev_ops  *bd_ops;

        /* Fields that should be ignored by drivers: */
        struct mmobj bd_mmobj;

        /* Link on the list of block-oriented devices */
        list_link_t bd_link;
} blockdev_t;

typedef struct blockdev_ops {
        /**
         * Reads a block from the block device. This call will block.
         *
         * @param bdev the block device
         * @param buf the memory into which to read the block (must be
         *      page-aligned)
         * @param loc the number of the block to start reading from
         * @param count the number of blocks to read
         * @return 0 on success, -errno on failure
         */
        int (*read_block)(blockdev_t *bdev, char *buf,
                          blocknum_t loc, size_t count);

        /**
         * Writes a block to the block device. This call will block.
         *
         * @param bdev the block device
         * @param buf the memory from which to write the block (must be
         *      page-aligned)
         * @param loc the number of the block to start writing at
         * @param count the number of blocks to write
         * @return 0 on success, -errno on failure
         */
        int (*write_block)(blockdev_t *bdev, const char *buf,
                           blocknum_t loc, size_t count);
} blockdev_ops_t;

/**
 * Initializes the block device subsystem.
 */
void blockdev_init(void);

/**
 * Registers a given block device.
 *
 * @param dev the block device to register
 */
int blockdev_register(blockdev_t *dev);

/**
 * Finds a block device with a given device id.
 *
 * @param id the device id of the block device to find
 * @return the block device with the given id if it exists, or NULL if
 * it cannot be found
 */
blockdev_t *blockdev_lookup(devid_t id);

/**
 * Cleans and frees all resident pages belonging to a given block
 * device.
 *
 * @param dev the block device to flush
 */
void blockdev_flush_all(blockdev_t *dev);
