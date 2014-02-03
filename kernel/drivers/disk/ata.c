#include "types.h"

#include "main/interrupt.h"
#include "main/io.h"

#include "util/string.h"
#include "util/debug.h"
#include "util/list.h"
#include "util/delay.h"

#include "drivers/blockdev.h"
#include "drivers/dev.h"
#include "drivers/pci.h"
#include "drivers/disk/dma.h"

#include "proc/sched.h"
#include "proc/kmutex.h"

#include "mm/kmalloc.h"
#include "mm/page.h"

/* Copied from the bochsrc, make sure it always matches */
#define IRQ_DISK_PRIMARY 14
#define IRQ_DISK_SECONDARY 15

/*
 * Huge pile of helpful definitions (copied from OSDev). Note that we
 * do not support busmaster, drive detection, or LBA48 We support
 * secondary channel and slave drive, but probably won't ever use
 * them.
 */

/* Interface type (we will only use ATA) */
#define ATA_TYPE_ATA   0x00
#define ATA_TYPE_ATAPI 0x01

/* Master or slave */
#define ATA_MASTER 0x00
#define ATA_SLAVE  0x01

/* Channels */
#define ATA_PRIMARY   0x00
#define ATA_SECONDARY 0x01

/* Operations */
#define ATA_READ  0x00
#define ATA_WRITE 0x01

/* "Base" port addresses */
/* The addresses to use for actual IO registers are offsets from these */
#define ATA_PRIMARY_CTRL_BASE    0x3f0
#define ATA_PRIMARY_CMD_BASE     0x1f0
#define ATA_SECONDARY_CTRL_BASE  0x370
#define ATA_SECONDARY_CMD_BASE   0x170
/* Note interrupt numbers defined in core/interrupt.h */

typedef void (*atac_intr_handler_t)(regs_t *regs, void *arg);

static struct ata_channel {
        /* Base port for cmd registers */
        uint16_t atac_cmd;

        /* Base port for ctrl registers */
        uint16_t atac_ctrl;

        /* Interrupt number for this channel */
        uint8_t atac_intr;

        /* Interrupt handler for this channel */
        atac_intr_handler_t atac_intr_handler;

        /* Argument for interrupt handler */
        void *atac_intr_arg;
	
	/* address of busmaster register */
	uint16_t atac_busmaster;
} ATA_CHANNELS[2] = {
        {
                ATA_PRIMARY_CMD_BASE,
                ATA_PRIMARY_CTRL_BASE,
                INTR_DISK_PRIMARY,
                NULL,
                NULL,
		NULL
        },
        {
                ATA_SECONDARY_CMD_BASE,
                ATA_SECONDARY_CTRL_BASE,
                INTR_DISK_SECONDARY,
                NULL,
                NULL,
		NULL
        }
};

#define ATA_NUM_CHANNELS 2

#define ATA_SECTOR_SIZE 512 /* Pretty much always true */

/* Drive/head values (for ATA_REG_DRIVEHEAD) */
#define ATA_DRIVEHEAD_MASTER 0xA0
#define ATA_DRIVEHEAD_SLAVE  0xB0

/* Port address offsets for registers */
/* Command registers */
#define ATA_REG_DATA       0x00 /* Data register (read/write address) */
#define ATA_REG_ERROR      0x01
#define ATA_REG_FEATURE    0x01
#define ATA_REG_SECCOUNT0  0x02 /* Number of sectors to read/write */
#define ATA_REG_SECNUM     0x03 /* chs addressing */
#define ATA_REG_CYLLOW     0x04
#define ATA_REG_CYLHIGH    0x05
#define ATA_REG_LBA0       0x03 /* lba addressing */
#define ATA_REG_LBA1       0x04
#define ATA_REG_LBA2       0x05
#define ATA_REG_DRIVEHEAD  0x06 /* Special drive info (used to set master/slave) */
#define ATA_REG_COMMAND    0x07 /* Write only */
#define ATA_REG_STATUS     0x07 /* Read only */
#define ATA_REG_SECCOUNT1  0x08 /* These four are only used in lba48 */
#define ATA_REG_LBA3       0x09
#define ATA_REG_LBA4       0x0A
#define ATA_REG_LBA5       0x0B /* --- */
#define ATA_REG_NIEN_CONTROL		 0x0C

/* Control registers */
#define ATA_REG_CONTROL    0x06 /* Write only */
/* Like status, but does not imply clear of interrupt */
#define ATA_REG_ALTSTATUS  0x06 /* Read only */
#define ATA_REG_DEVADDRESS 0x07

/* Status codes (for ATA_REG_STATUS) */
#define ATA_SR_BSY  0x80 /* Busy */
#define ATA_SR_DRDY 0x40 /* Drive ready */
#define ATA_SR_DF   0x20 /* Drive write fault */
#define ATA_SR_DSC  0x10 /* Drive seek complete */
#define ATA_SR_DRQ  0x08 /* Data request ready */
#define ATA_SR_CORR 0x04 /* Corrected data */
#define ATA_SR_IDX  0x02 /* inlex */
#define ATA_SR_ERR  0x01 /* Error */

/* Error codes (for ATA_REG_ERROR) */
#define ATA_ER_BBK   0x80 /* Bad sector */
#define ATA_ER_UNC   0x40 /* Uncorrectable data */
#define ATA_ER_MC    0x20 /* No media */
#define ATA_ER_IDNF  0x10 /* ID mark not found */
#define ATA_ER_MCR   0x08 /* No media */
#define ATA_ER_ABRT  0x04 /* Command aborted */
#define ATA_ER_TK0NF 0x02 /* Track 0 not found */
#define ATA_ER_AMNF  0x01 /* No address mark */

/* Commands (for ATA_REG_COMMAND) */
#define ATA_CMD_READ_PIO        0x20
#define ATA_CMD_READ_PIO_EXT    0x24
#define ATA_CMD_READ_DMA        0xC8
#define ATA_CMD_READ_DMA_EXT    0x25
#define ATA_CMD_WRITE_PIO       0x30
#define ATA_CMD_WRITE_PIO_EXT   0x34
#define ATA_CMD_WRITE_DMA       0xCA
#define ATA_CMD_WRITE_DMA_EXT   0x35
#define ATA_CMD_CACHE_FLUSH     0xE7
#define ATA_CMD_CACHE_FLUSH_EXT 0xEA
#define ATA_CMD_PACKET          0xA0
#define ATA_CMD_IDENTIFY_PACKET 0xA1
#define ATA_CMD_IDENTIFY        0xEC

/* Drive/head values for CHS / LBA */
#define ATA_DRIVEHEAD_CHS 0x00
#define ATA_DRIVEHEAD_LBA 0x40

#define ATA_IDENT_MAX_LBA 30

/* Reads from the command registers, NOT the control registers */
#define ata_inb_reg(channel, reg) inb(ATA_CHANNELS[channel].atac_cmd + reg)
#define ata_inw_reg(channel, reg) inw(ATA_CHANNELS[channel].atac_cmd + reg)
#define ata_inl_reg(channel, reg) inl(ATA_CHANNELS[channel].atac_cmd + reg)

/* Writes to command registers */
#define ata_outb_reg(channel, reg, data) \
        outb(ATA_CHANNELS[(channel)].atac_cmd + (reg), (data))
#define ata_outw_reg(channel, reg, data) \
        outw(ATA_CHANNELS[(channel)].atac_cmd + (reg), (data))
#define ata_outl_reg(channel, reg, data) \
        outl(ATA_CHANNELS[(channel)].atac_cmd + (reg), (data))

/* Helpful for delaying, etc. */
#define ata_inb_altstatus(channel) \
        inb(ATA_CHANNELS[(channel)].atac_ctrl + ATA_REG_ALTSTATUS)

/* Delay 1 port read without implying interrupt clear (as reading from
 * status does) */
static void
ata_sync(uint8_t channel)
{
        ata_inb_altstatus(channel);
}

/* Sync and wait a bit */
static void
ata_pause(uint8_t channel)
{
        ata_sync(channel);
        ndelay(400);
}

#define ATA_IDENT_BUFSIZE 256

#define bd_to_ata(bd) (CONTAINER_OF((bd), ata_disk_t, ata_bdev))


typedef struct ata_disk {
        /* 0 (Primary Channel) or 1 (Secondary Channel) */
        uint8_t    ata_channel;

        /* 0 (Master Drive) or 1 (Slave Drive) */
        uint8_t    ata_drive;

        /* Size of disk in number of sectors */
        uint32_t   ata_size;

        uint32_t   ata_sectors_per_block;

        /* Threads making blocking disk operations wait one this
         * queue, and disk interrupt wakes them up */
        ktqueue_t  ata_waitq;

        /* Disk mutex since only one process can be using the disk at
         * any time */
        kmutex_t   ata_mutex;

        /* Underlying block device */
        blockdev_t ata_bdev;
} ata_disk_t;

/* this prototype needs to be after the struct definition */
uint16_t ata_setup_busmaster(ata_disk_t* adisk);

#define NDISKS __NDISKS__

static void ata_intr_wrapper(regs_t *regs);
static int ata_read(blockdev_t *bdev, char *data,
                    blocknum_t blocknum, unsigned int count);
static int ata_write(blockdev_t *bdev, const char *data,
                     blocknum_t blocknum, unsigned int count);
static int ata_do_operation(ata_disk_t *adisk, char *data, \
                            blocknum_t sectornum, int write);
static void ata_intr(regs_t *regs, void *arg);

static blockdev_ops_t ata_disk_ops = {
        .read_block  = ata_read,
        .write_block = ata_write
};

void
ata_init()
{
        int ii;

        intr_map(IRQ_DISK_PRIMARY, INTR_DISK_PRIMARY);
        intr_map(IRQ_DISK_SECONDARY, INTR_DISK_SECONDARY);

        dma_init(); /* IMPORTANT! */

        uint8_t oldipl = intr_getipl();
        intr_setipl(INTR_DISK_PRIMARY);

        for (ii = 0; ii < NDISKS; ii++) {
                int i;
                int err = 0;
                uint32_t ident_buf[ATA_IDENT_BUFSIZE];
                uint8_t status = 0;
                int channel = ii; /* No slave drive support */
                ata_disk_t *adisk;

                if (ii >= ATA_NUM_CHANNELS)
                        panic("ATA does not have as many drives"
                              "as you want!\n");
                /* Choose drive - In this case always Master */
                ata_outb_reg(channel, ATA_REG_DRIVEHEAD, ATA_DRIVEHEAD_MASTER | ATA_DRIVEHEAD_LBA);
                /* Set the Sector count register to be 0 */
                ata_outb_reg(channel, ATA_REG_SECCOUNT0, 0);
                /* Set the LBA0 LBA1 LBA2 registers to be 0 */
                ata_outb_reg(channel, ATA_REG_LBA0, 0);
                ata_outb_reg(channel, ATA_REG_LBA1, 0);
                ata_outb_reg(channel, ATA_REG_LBA2, 0);

                /* disable IRQs for the master (shamelessly stolen from OS Dev */
                outb(ATA_PRIMARY_CTRL_BASE + ATA_REG_CONTROL, 0x02);
		
                /* Tell drive to get ready to in identification space */
                ata_outb_reg(channel, ATA_REG_COMMAND, ATA_CMD_IDENTIFY);

                /* wait some time for the drive to process */
                ata_pause(channel);

                /* If status register is 0xff, drive does not exist */
                if (0x00 == ata_inb_reg(channel, ATA_REG_STATUS)) {
                  dbgq(DBG_DISK, "Drive does not exist\n");
                  continue;
                }

                /* poll until the BSY bit clears */
                while(1) {
                  status = ata_inb_reg(channel, ATA_REG_STATUS);
                  if (!(status & ATA_SR_BSY)) break;
                  ata_pause(channel);	
                }

                /* Now the drive is no longer busy, poll until the error bit is set or drq is set */
                while (1) {
                  status = ata_inb_reg(channel, ATA_REG_STATUS);
                  if (status & ATA_SR_ERR) { err = 1; break; }
                  if (status & ATA_SR_DRQ) break;
                  ata_pause(channel);
                }

                if (err) {
                  panic("Error setting up ATA drive\n");
                }

                /* Now clear the command register */
                outb(ATA_PRIMARY_CTRL_BASE + ATA_REG_CONTROL, 0x00);

                /* Otherwise, allocate new disk */
                if (NULL ==
                    (adisk = (ata_disk_t *)kmalloc(sizeof(ata_disk_t))))
                        panic("Not enough memory for ata disk struct!\n");
                adisk->ata_channel = channel;
                adisk->ata_drive = 0;

                for (i = 0; i < ATA_IDENT_BUFSIZE; i++) {
                        ident_buf[i] = ata_inl_reg(adisk->ata_channel, ATA_REG_DATA);
                }
                /* Determine disk size */
                adisk->ata_size = ident_buf[ATA_IDENT_MAX_LBA];
                /* In theory we could use this identification buffer
                 * to find out lots of other things but we don't
                 * really need to know any of them */

                adisk->ata_sectors_per_block = BLOCK_SIZE / ATA_SECTOR_SIZE;

                sched_queue_init(&adisk->ata_waitq);
                kmutex_init(&adisk->ata_mutex);

                dbg(DBG_DISK, "Initialized ATA device %d, channel %s, drive %s, size %d\n",
                    ii, (adisk->ata_channel ? "SECONDARY" : "PRIMARY"),
                    (adisk->ata_drive ? "SLAVE" : "MASTER"), adisk->ata_size);

                /* Set up corresponding handler */
                intr_register(ATA_CHANNELS[adisk->ata_channel].atac_intr,
                              ata_intr_wrapper);
                ATA_CHANNELS[adisk->ata_channel].atac_intr_handler = ata_intr;
                ATA_CHANNELS[adisk->ata_channel].atac_intr_arg = adisk;
                ATA_CHANNELS[adisk->ata_channel].atac_busmaster = ata_setup_busmaster(adisk);

                adisk->ata_bdev.bd_id = MKDEVID(DISK_MAJOR, ii);
                adisk->ata_bdev.bd_ops = &ata_disk_ops;
                blockdev_register(&adisk->ata_bdev);
        }
        intr_setipl(oldipl);
}

static void
ata_intr_wrapper(regs_t *regs)
{
        int i;
        dbg(DBG_DISK, "ATA interrupt\n");
        for (i = 0; i < ATA_NUM_CHANNELS; i++) {
                /* Check if interrupt is for this channel */
                if (ATA_CHANNELS[i].atac_intr == regs->r_intr) {
                        if (NULL == ATA_CHANNELS[i].atac_intr_handler)
                                panic("No handler registered "
                                      "for ATA channel %d!\n", i);
                        ATA_CHANNELS[i].atac_intr_handler(
                                regs, ATA_CHANNELS[i].atac_intr_arg);
                        /* Acknowledge interrupt */
                        ata_inb_reg(i, ATA_REG_STATUS);
                        return;
                }
        }
        panic("Received interrupt on channel we don't know about\n");
}

/**
 * Reads a given number of blocks from a block device starting at a
 * given block number into a buffer.
 *
 * @param bdev the block device to read from
 * @param data buffer to write to
 * @param blocknum the block number to start reading at
 * @param count the number of blocks to read
 * @return 0 on success and <0 on error
 */
static int
ata_read(blockdev_t *bdev, char *data, blocknum_t blocknum, unsigned int count)
{
        NOT_YET_IMPLEMENTED("DRIVERS: ata_read");
        return -1;
}

/**
 * Writes a a given number of blocks from a buffer to a block device
 * starting at a given block.
 *
 * @param bdev the block device to write to
 * @param data buffer to read data from
 * @param blocknum the block number to start writing at
 * @param count the number of blocks to write
 * @return 0 on success and <0 on error
 */
static int
ata_write(blockdev_t *bdev, const char *data, blocknum_t blocknum, unsigned int count)
{
        NOT_YET_IMPLEMENTED("DRIVERS: ata_write");
        return -1;
}

/**
 * Read/write the given block.
 *
 * @param adisk the disk to perform the operation on
 * @param data the buffer to write from or read into
 * @param blocknum which block on the disk to read or write
 * @param write true if writing, false if reading
 * @return 0 on sucess or <0 on error
 */
/*
 * In this function you will perform a disk operation using
 * direct memory access (DMA). Follow these steps _VERY_
 * carefully. The steps are as follows:
 *
 *     o Lock the mutex and set the IPL. We don't want to
 *     other threads trying to perform an operation while we
 *     are in the middle of this operation. We also don't want
 *     to receive a disk interrupt, which is supposed to wake
 *     up this thread to alert us that the DMA operation has
 *     completed, before the thread goes to sleep and puts
 *     itself on the wait queue. Since the only interrupts we
 *     care about are disk interrupts, we do not have to mask
 *     all interrupts, just disk interrupts (and consequently,
 *     all interrupts with lower priority than disk
 *     interrupts). Try INTR_DISK_SECONDARY.
 *
 *     o Initialize DMA for this operation (see the dma_load()
 *     function)
 *
 *     o Write to the disk's registers to tell it the number
 *     of sectors that will be read/writen and the starting
 *     sector. Use the ata_outb_reg() function for writing
 *     to the hardware registers (as these are each one byte
 *     in size).
 *
 *     We use logical block addressing (LBA) to specify the
 *     starting sector. Our interface supports 24-bit sector
 *     numbers, and up to 256* sectors to be read/written at a
 *     time. Write the number of sectors to ATA_REG_SECCOUNT0.
 *     Write the sector number in little-endian order to
 *     ATA_REG_LBA{0-2} (least-significant eight bits to
 *     ATA_REG_LBA0, middle eight bits to ATA_REG_LBA1,
 *     most-significant eight bits to ATA_REG_LBA2).
 *
 *     (* Note that the special value 0 when written to this
 *     register will in fact write 256 sectors, though you will
 *     not ever be writing this value)
 *
 *     o Write to the disk's registers to tell it the type of
 *     operation it will be performing.
 *
 *     You should write either ATA_CMD_WRITE_DMA or
 *     ATA_CMD_READ_DMA to the ATA_REG_COMMAND register.
 *
 *     o Pause to make sure the disk is ready to go (see the
 *     ata_pause() function).
 *
 *     o Start the DMA operation (see the dma_start() function).
 *
 *     o Now that we have given the disk and the DMA
 *     controller the necessary information, all we have to do
 *     is wait. This is the whole point of DMA: this process
 *     can yield, allowing the CPU to perform other tasks
 *     while we wait for the disk to seek and perform the
 *     requested operation.
 *
 *     Specifically, we want to sleep on the disk's wait queue
 *     so we can be woken up by the interrupt handler, which
 *     will be called when the DMA operation is completed.
 *
 *     o Once we have woken up from sleep, we need to read
 *     the status of the DMA operation from the disk's
 *     ATA_REG_STATUS register (see the ata_inb_reg()
 *     function).
 *
 *     o Next, we check the status to see if the error bit is
 *     set (for status flags, see ATA_SR_* values). If there
 *     is an error, read the error code from the disk's
 *     ATA_REG_ERROR register (propagate it up as -error).
 *
 *     o Alert the DMA controller that we have received the
 *     interrupt and, if necessary, clear the error bit (see
 *     dma_reset() function).
 *
 *     o Now we are finished. Restore the IPL, release any
 *     locks we have, and return the status of the DMA
 *     operation.
 */
static int
ata_do_operation(ata_disk_t *adisk, char *data, blocknum_t blocknum, int write)
{
        NOT_YET_IMPLEMENTED("DRIVERS: ata_do_operation");
        return -1;
}

/**
 * Interrupt handler called by the disk when an operation has
 * completed.
 *
 * @param regs the register state
 * @param arg the disk the operation was performed on. This should be
 * a pointer to an ata_disk_t struct.
 */
static void
ata_intr(regs_t *regs, void *arg)
{
        NOT_YET_IMPLEMENTED("DRIVERS: ata_intr");
}

/*
 * This function takes in an ATA Disk struct and 
 * sets it up for busmastering DMA
 */
uint16_t ata_setup_busmaster(ata_disk_t* adisk) {
  /* First step is to read the command register and see what's there */
	pcidev_t* ide = pci_lookup(0x01, 0x01, 0x80);

	if (ide == NULL) {
		panic("Could not find ide device\n");
	}

	uint32_t command = pci_read_config(ide, PCI_COMMAND, 2); 
	/* set the busmaster bit to 1 to enable busmaster */
	command |= 0x4;
	/* clear bit 10 to make sure that interrupts are enabled */
	command &= 0xfdff;
	
	pci_write_config(ide, PCI_COMMAND, command, 2);
	/* read BAR4 and return the address of the busmaster register */
	uint32_t busmaster_base = ide->pci_bar[4].base_addr + (adisk->ata_channel * 8);
	
	if (busmaster_base == 0) {
		panic("No valid busmastering address\n");
	}

	KASSERT(busmaster_base != 0 && "Disk device should not have 0 for the busmaster register");

	return (uint16_t)busmaster_base;
}
