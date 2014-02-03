import os
import math
import struct

S5_MAGIC = 0x727f
S5_CURRENT_VERSION = 3
S5_BLOCK_SIZE = 4096

S5_NBLKS_PER_FNODE = 30
S5_NDIRECT_BLOCKS = 28
S5_MAX_FILE_BLOCKS = S5_NDIRECT_BLOCKS + math.floor(S5_BLOCK_SIZE / 4)
S5_MAX_FILE_SIZE = S5_MAX_FILE_BLOCKS * S5_BLOCK_SIZE

S5_NAME_LEN = 28
S5_DIRENT_SIZE = S5_NAME_LEN + 4

S5_INODE_SIZE = 16 + S5_NDIRECT_BLOCKS * 4
S5_INODES_PER_BLOCK = S5_BLOCK_SIZE / S5_INODE_SIZE

S5_TYPE_FREE = 0x0
S5_TYPE_DATA = 0x1
S5_TYPE_DIR = 0x2
S5_TYPE_CHR = 0x4
S5_TYPE_BLK = 0x8
S5_TYPES = set([ S5_TYPE_FREE, S5_TYPE_DATA, S5_TYPE_DIR, S5_TYPE_CHR, S5_TYPE_BLK ])

class S5fsException(Exception):

    def __init__(self, msg):
        self._msg = msg

    def __str__(self):
        return self._msg

class S5fsDiskSpaceException(S5fsException):

    def __init__(self):
        S5fsException.__init__(self, "out of disk space")

class Block:

    def __init__(self, simdisk, offset, blockno):
        self._simdisk = simdisk
        self._offset = offset
        self._blockno = blockno

    def get_blockno(self):
        return self._blockno

    def read(self, offset=0, size=None):
        if (size == None):
            size = S5_BLOCK_SIZE - offset
        if (offset + size > S5_BLOCK_SIZE):
            raise S5fsException("cannot read to {0}, exceeds block size of {1}".format(offset + size, S5_BLOCK_SIZE))
        self._simdisk._simfile.seek(int(self._offset + offset))
        res = self._simdisk._simfile.read(size)
        return res

    def write(self, offset, data):
        if (offset + len(data) > S5_BLOCK_SIZE):
            raise S5fsException("cannot write to {0}, exceeds block size of {1}".format(offset + len(data), S5_BLOCK_SIZE))
        self._simdisk._simfile.seek(int(self._offset + offset))
        res = self._simdisk._simfile.write(data)

    def zero(self):
        self._simdisk._simfile.seek(self._offset)
        for i in xrange(S5_BLOCK_SIZE):
            self._simdisk._simfile.write('\0')

    def free(self):
        if (self._simdisk.get_nfree() < S5_NBLKS_PER_FNODE - 1):
            self._simdisk.set_free_block(self._simdisk.get_nfree(), self._blockno)
            self._simdisk.set_nfree(self._simdisk.get_nfree() + 1)
        else:
            for i in xrange(S5_NBLKS_PER_FNODE - 1):
                self.write(i * 4, struct.pack("I", self._simdisk.get_free_block(i)))
            self.write((S5_NBLKS_PER_FNODE - 1) * 4, struct.pack("I", self._simdisk.get_last_free_block()))
            self._simdisk.set_last_free_block(self._blockno)
            self._simdisk.set_nfree(0)

class Dirent:
    
    def __init__(self, parent, inode, name, offset):
        self._parent = parent
        self.inode = inode
        self.name = name
        self._offset = offset

    def remove(self):
        self._parent.write(self._offset + 4, '\0')

class Inode:

    def __init__(self, simdisk, number, offset):
        self._simdisk = simdisk
        self._simfile = simdisk._simfile
        self._number = number
        self._offset = offset

    def get_next_free(self):
        return self.get_size()

    def set_next_free(self, val):
        return self.set_size(val)

    def get_size(self):
        self._simfile.seek(int(self._offset))
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_size(self, val):
        self._simfile.seek(int(self._offset))
        self._simfile.write(struct.pack("I", val))

    def get_number(self):
        self._simfile.seek(int(self._offset + 4))
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_number(self, val):
        self._simfile.seek(int(self._offset + 4))
        self._simfile.write(struct.pack("I", val))

    def get_type(self):
        self._simfile.seek(int(self._offset + 8))
        return struct.unpack("H", self._simfile.read(2))[0]

    def set_type(self, val):
        self._simfile.seek(int(self._offset + 8))
        self._simfile.write(struct.pack("H", val))

    def get_link_count(self):
        self._simfile.seek(int(self._offset + 10))
        return struct.unpack("h", self._simfile.read(2))[0]

    def set_link_count(self, val):
        self._simfile.seek(int(self._offset + 10))
        self._simfile.write(struct.pack("h", val))

    def get_direct_blockno(self, index):
        if (index < S5_NDIRECT_BLOCKS):
            self._simfile.seek(int(self._offset + 12 + index * 4))
            return struct.unpack("I", self._simfile.read(4))[0]
        else:
            raise S5fsException("direct block index {0} greater than max {1}".format(index, S5_NDIRECT_BLOCKS))

    def set_direct_blockno(self, index, val):
        if (index < S5_NDIRECT_BLOCKS):
            self._simfile.seek(int(self._offset + 12 + index * 4))
            self._simfile.write(struct.pack("I", val))
        else:
            raise S5fsException("direct block index {0} greater than max {1}".format(index, S5_NDIRECT_BLOCKS))

    def get_indirect_blockno(self):
        self._simfile.seek(int(self._offset + 12 + 4 * S5_NDIRECT_BLOCKS))
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_indirect_blockno(self, val):
        self._simfile.seek(int(self._offset + 12 + 4 * S5_NDIRECT_BLOCKS))
        self._simfile.write(struct.pack("I", val))

    def get_type_str(self, short=False):
        t = self.get_type()
        name = "INV" if short else "INVALID"
        if (t == S5_TYPE_FREE):
            name = "---" if short else "S5_TYPE_FREE"
        elif (t == S5_TYPE_DATA):
            name = "dat" if short else "S5_TYPE_DATA"
        elif (t == S5_TYPE_DIR):
            name = "dir" if short else "S5_TYPE_DIR"
        elif (t == S5_TYPE_BLK):
            name = "blk" if short else "S5_TYPE_BLK"
        elif (t == S5_TYPE_CHR):
            name = "chr" if short else "S5_TYPE_CHR"
        return name if short else "{0} (0x{1:02x})".format(name, t)

    def get_summary(self):
        res = ""
        res += "num:   {0}{1}\n".format(self.get_number(), "" if self.get_number() == self._number else " (INVALID, should be {0})".format(self.get_number()))
        res += "type:  {0}\n".format(self.get_type_str())
        if (self.get_type() != S5_TYPE_FREE):
            res += "links: {0}\n".format(self.get_link_count())
        if (self.get_type() in set([ S5_TYPE_DATA, S5_TYPE_DIR ])):
            res += "size:  {0} bytes".format(self.get_size())
            if (self.get_size() > S5_MAX_FILE_SIZE):
                res += " (INVALID, max file size is {0})".format(S5_MAX_FILE_SIZE)
            elif (self.get_type() == S5_TYPE_DIR and self.get_size() % S5_DIRENT_SIZE != 0):
                res += " (INVALID, directory size must be multiple of dirent size ({0}))".format(S5_DIRENT_SIZE)
            elif (self.get_type() == S5_TYPE_DIR):
                res += " ({0} dirents)".format(self.get_size() / S5_DIRENT_SIZE)
            res += "\n"
            res += "direct blocks ({0}):\n".format(S5_NDIRECT_BLOCKS)
            for i in xrange(S5_NDIRECT_BLOCKS):
                res += " {0:5}".format(self.get_direct_blockno(i))
                if ((i + 1) % 4 == 0):
                    res += "\n"
            if (res[-1] != "\n"):
                res += "\n"
            res += "indirect block: {0}\n".format(self.get_indirect_blockno())
        elif (self.get_type() == S5_TYPE_FREE):
            res += "next free: {0}\n".format(self.get_next_free())
        res = res[:-1]
        return res

    def read(self, offset=0, size=None):
        if (size == None):
            size = self.get_size()
        if (self.get_type() not in set([ S5_TYPE_DATA, S5_TYPE_DIR ])):
            raise S5fsException("cannot read from inode of type " + self.get_type_str())
        size = min(size, min(S5_MAX_FILE_SIZE, self.get_size()) - offset)
        res = ""
        while (size > 0):
            blockno = math.floor(offset / S5_BLOCK_SIZE)
            blockoff = offset % S5_BLOCK_SIZE
            ammount = min(S5_BLOCK_SIZE - blockoff, size)
            if (blockno < S5_NDIRECT_BLOCKS):
                blockno = self.get_direct_blockno(blockno)
            else:
                if (self.get_indirect_blockno() == 0):
                    blockno = 0
                else:
                    indirect = self._simdisk.get_block(self.get_indirect_blockno())
                    blockno -= S5_NDIRECT_BLOCKS
                    blockno = struct.unpack("I", indirect.read(blockno * 4, 4))[0]
            if (blockno == 0):
                for i in xrange(ammount):
                    res += '\0'
            else:
                res += self._simdisk.get_block(blockno).read(blockoff, ammount)
            offset += ammount
            size -= ammount
        return res

    def write(self, offset, data):
        if (self.get_type() not in set([ S5_TYPE_DATA, S5_TYPE_DIR ])):
            raise S5fsException("cannot write to inode of type " + self.get_type_str())
        if (offset + len(data) > S5_MAX_FILE_SIZE):
            raise S5fsException("cannot write up to byte {0}, max file size is {1}".format(offset + len(data), S5_MAX_FILE_SIZE))
        remaining = len(data)
        while (remaining > 0):
            blockloc = math.floor(offset / S5_BLOCK_SIZE)
            blockoff = offset % S5_BLOCK_SIZE
            ammount = min(S5_BLOCK_SIZE - blockoff, remaining)
            if (blockloc < S5_NDIRECT_BLOCKS):
                blockno = self.get_direct_blockno(blockloc)
            else:
                if (self.get_indirect_blockno() == 0):
                    indirect = self._simdisk.alloc_block()
                    indirect.zero()
                    self.set_indirect_blockno(indirect.get_blockno())
                    blockno = 0
                else:
                    indirect = self._simdisk.get_block(self.get_indirect_blockno())
                    blockno = struct.unpack("I", indirect.read((blockloc - S5_NDIRECT_BLOCKS) * 4, 4))[0]
            if (blockno == 0):
                block = self._simdisk.alloc_block()
                block.zero()
                if (blockloc < S5_NDIRECT_BLOCKS):
                    self.set_direct_blockno(blockloc, block.get_blockno())
                else:
                    indirect = self._simdisk.get_block(self.get_indirect_blockno())
                    indirect.write((blockloc - S5_NDIRECT_BLOCKS) * 4, struct.pack("I", block.get_blockno()))
            else:
                block = self._simdisk.get_block(blockno)
            if (remaining == ammount):
                block.write(blockoff, data[-remaining:])
            else:
                block.write(blockoff, data[-remaining:-remaining+ammount])
            remaining -= ammount
            offset += ammount
        if (offset > self.get_size()):
            self.set_size(offset)

    def truncate(self, size=0):
        target = math.floor((size - 1) / S5_BLOCK_SIZE)
        curr = math.floor(self.get_size() / S5_BLOCK_SIZE)
        while (curr > target):
            if (curr < S5_NDIRECT_BLOCKS):
                blockno = self.get_direct_blockno(curr)
            else:
                if (self.get_indirect_blockno() == 0):
                    blockno = 0
                else:
                    indirect = self._simdisk.get_block(self.get_indirect_blockno())
                    blockno = struct.unpack("I", indirect.read((curr - S5_NDIRECT_BLOCKS) * 4, 4))[0]
            if (blockno > 0):
                block = self._simdisk.get_block(blockno)
                block.free()
                if (curr < S5_NDIRECT_BLOCKS):
                    self.set_direct_blockno(curr, 0)
                else:
                    indirect = self._simdisk.get_block(self.get_indirect_blockno())
                    indirect.write((curr - S5_NDIRECT_BLOCKS) * 4, struct.pack("I", 0))
                    if (curr == S5_NDIRECT_BLOCKS):
                        indirect.free()
                        self.set_indirect_blockno(0)
            curr -= 1
        while (curr < target):
            if (curr < S5_NDIRECT_BLOCKS):
                self.set_direct_blockno(0)
            else:
                if (self.get_indirect_blockno() != 0):
                    indirect = self._simdisk.get_block(self.get_indirect_blockno())
                    indirect.write((curr - S5_NDIRECT_BLOCKS) * 4, struct.pack("I", 0))
        self.set_size(size)

    def _find_dirent(self, name, types=S5_TYPES):
        if (self.get_type() != S5_TYPE_DIR):
            raise S5fsException("cannot remove directory entry in non-directory inode of type " + self.get_type_str())
        if (self.get_size() % S5_DIRENT_SIZE != 0):
            raise S5fsException("cannot remove directory entry, inode has size {0} not a multiple of dirent size {1}".format(self.get_size(), S5_DIRENT_SIZE))
        if (len(name) >= S5_NAME_LEN):
            raise S5fsException("directroy entry name '{0}' too long, limit is {1} characters".format(name, S5_NAME_LEN - 1))
        for i in xrange(0, self.get_size(), S5_DIRENT_SIZE):
            inode = struct.unpack("I", self.read(i, 4))[0]
            parts = self.read(i + 4, S5_NAME_LEN).split('\0', 1)
            if (len(parts) == 1):
                raise S5fsException("directory entry name '{0}' does not contain a null character".format(name))
            if (name == parts[0]):
                return Dirent(self, inode, name, i)
        return None

    def unlink(self, name):
        dirent = self._find_dirent(name)
        if (dirent == None):
            raise S5fsException("no such file or directory: {0}".format(name))
        inode = self._simdisk.get_inode(dirent.inode)
        if (inode.get_type() == S5_TYPE_DIR):
            raise S5fsException("cannot unlink directory: {0}".format(name))
        if (inode.get_link_count() < 1):
            raise S5fsException("inode being unlinked has invalid link count of {1}: {0}".format(name, inode.get_link_count()))
        dirent.remove()
        if (inode.get_link_count() == 1):
            inode.set_link_count(0)
            inode.free()
        else:
            inode.set_link_count(inode.get_link_count() - 1)

    def rmdir(self, name):
        if (name == "." or name == ".."):
            raise S5fsException("cannot rmdir special directory: {0}".format(name))
        dirent = self._find_dirent(name)
        if (dirent == None):
            raise S5fsException("no such file or directory: {0}".format(name))
        inode = self._simdisk.get_inode(dirent.inode)
        if (inode.get_type() != S5_TYPE_DIR):
            raise S5fsException("cannot rmdir non-directory: {0}".format(name))
        if (not inode.is_empty()):
            raise S5fsException("cannot rmdir non-empty directory: {0}".format(name))
        if (inode.get_link_count() != 1):
            raise S5fsException("empty directory has link count of {1}: {0}".format(name, inode.get_link_count()))
        dirent.remove()
        self.set_link_count(self.get_link_count() - 1)
        inode.set_link_count(0)
        inode.free()

    def _make_dirent(self, inode, name):
        if (self.get_type() != S5_TYPE_DIR):
            raise S5fsException("cannot create directory entry in non-directory inode of type " + self.get_type_str())
        if (self.get_size() % S5_DIRENT_SIZE != 0):
            raise S5fsException("cannot create directory entry, inode has size {0} not a multiple of dirent size {1}".format(self.get_size(), S5_DIRENT_SIZE))
        if (len(name) >= S5_NAME_LEN):
            raise S5fsException("directroy entry name '{0}' too long, limit is {1} characters".format(name, S5_NAME_LEN - 1))
        empty = -1
        for i in xrange(0, self.get_size(), S5_DIRENT_SIZE):
            direntname = self.read(i + 4, S5_NAME_LEN).split('\0', 1)[0]
            if (direntname == name):
                raise S5fsException("directory already has entry with same name: {0}".format(name))
            if (len(name) == 0):
                empty = i
        if (empty >= 0):
            self.write(empty, struct.pack("I", inode))
            self.write(empty + 4, name.ljust(S5_NAME_LEN, '\0'))
        else:
            self.write(self.get_size(), struct.pack("I", inode))
            self.write(self.get_size(), name.ljust(S5_NAME_LEN, '\0'))

    def create(self, name):
        inode = self._simdisk.alloc_inode()
        try:
            inode.set_type(S5_TYPE_DATA)
            inode.set_size(0)
            inode.set_link_count(1)
            for i in xrange(S5_NDIRECT_BLOCKS):
                inode.set_direct_blockno(i, 0)
            inode.set_indirect_blockno(0)
            self._make_dirent(inode.get_number(), name)
            return inode
        except S5fsException as e:
            inode.free()
            raise e

    def mkdir(self, name):
        inode = self._simdisk.alloc_inode()
        try:
            inode.set_type(S5_TYPE_DIR)
            inode.set_size(0)
            inode.set_link_count(1)
            for i in xrange(S5_NDIRECT_BLOCKS):
                inode.set_direct_blockno(i, 0)
            inode.set_indirect_blockno(0)
            inode._make_dirent(inode.get_number(), ".")
            inode._make_dirent(self.get_number(), "..")
            self.set_link_count(self.get_link_count() + 1)
            self._make_dirent(inode.get_number(), name)
            return inode
        except S5fsException as e:
            inode.free()
            raise e

    def getdents(self):
        if (self.get_type() != S5_TYPE_DIR):
            raise S5fsException("cannot get dirents from inode of type " + self.get_type_str())
        if (self.get_size() % S5_DIRENT_SIZE != 0):
            raise S5fsException("cannot get dirents, inode has size {0} not a multiple of dirent size {1}".format(self.get_size(), S5_DIRENT_SIZE))
        for i in xrange(0, self.get_size(), S5_DIRENT_SIZE):
            inode = struct.unpack("I", self.read(i, 4))[0]
            name = self.read(i + 4, S5_NAME_LEN)
            parts = name.split('\0', 1)
            if (len(parts) == 1):
                raise S5fsException("directory entry {0} in inode {1} does not contain a null character".format(i / S5_DIRENT_SIZE, self._number))
            name = parts[0]
            if (len(name) > 0):
                yield Dirent(self, inode, name, i)

    def is_empty(self):
        for dirent in self.getdents():
            if (dirent.name != "." and dirent.name != ".."):
                return False
        return True

    def lookup(self, name):
        if (len(name) >= S5_NAME_LEN):
            raise S5fsException("directroy entry name '{0}' too long, limit is {1} characters".format(name, S5_NAME_LEN - 1))
        for dirent in self.getdents():
            if (dirent.name == name):
                try:
                    return self._simdisk.get_inode(dirent.inode)
                except S5fsException as e:
                    raise S5fsException("error looking up '{0}': {1}".format(name, str(e)))
        return None

    def open(self, path, create=False):
        opath = os.path.normpath(path)
        path = opath.split("/")
        curr = self
        for seg in path[:-1]:
            if (len(seg) == 0):
                continue
            curr = curr.lookup(seg)
            if (curr == None):
                if (not create):
                    return None
                else:
                    raise S5fsException("segment '{0}' in path '{1}' does not exist".format(seg, opath))
        if (len(path[-1]) == 0):
            return curr
        last = curr.lookup(path[-1])
        if (None == last and create):
            return curr.create(path[-1])
        return last

    def free(self):
        if (self.get_size() != 0):
            self.truncate()
        self.set_type(S5_TYPE_FREE)
        self.set_next_free(self._simdisk.get_free_inode())
        self._simdisk.set_free_inode(self._number)

class Simdisk:

    def __init__(self, simfile):
        self._simfile = simfile

    def get_magic(self):
        self._simfile.seek(0)
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_magic(self, val):
        self._simfile.seek(0)
        self._simfile.write(struct.pack("I", val))

    def get_free_inode(self):
        self._simfile.seek(4)
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_free_inode(self, val):
        self._simfile.seek(4)
        self._simfile.write(struct.pack("I", val))

    def get_nfree(self):
        self._simfile.seek(8)
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_nfree(self, val):
        self._simfile.seek(8)
        self._simfile.write(struct.pack("I", val))

    def get_free_block(self, index):
        if (index < S5_NBLKS_PER_FNODE - 1):
            self._simfile.seek(12 + 4 * index)
            return struct.unpack("I", self._simfile.read(4))[0]
        else:
            raise S5fsException("free block index {0} greater than max {1}".format(index, S5_NBLKS_PER_FNODE - 2))

    def set_free_block(self, index, val):
        if (index < S5_NBLKS_PER_FNODE - 1):
            self._simfile.seek(12 + 4 * index)
            self._simfile.write(struct.pack("I", val))
        else:
            raise S5fsException("free block index {0} greater than max {1}".format(index, S5_NBLKS_PER_FNODE - 2))

    def get_last_free_block(self):
        self._simfile.seek(12 + 4 * (S5_NBLKS_PER_FNODE - 1))
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_last_free_block(self, val):
        self._simfile.seek(12 + 4 * (S5_NBLKS_PER_FNODE - 1))
        self._simfile.write(struct.pack("I", val))

    def get_root_inode(self):
        self._simfile.seek(12 + 4 * S5_NBLKS_PER_FNODE)
        return struct.unpack("I", self._simfile.read(4))[0]

    def get_num_inodes(self):
        self._simfile.seek(16 + 4 * S5_NBLKS_PER_FNODE)
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_num_inodes(self, val):
        self._simfile.seek(16 + 4 * S5_NBLKS_PER_FNODE)
        self._simfile.write(struct.pack("I", val))

    def get_version(self):
        self._simfile.seek(20 + 4 * S5_NBLKS_PER_FNODE)
        return struct.unpack("I", self._simfile.read(4))[0]

    def set_version(self, val):
        self._simfile.seek(20 + 4 * S5_NBLKS_PER_FNODE)
        self._simfile.write(struct.pack("I", val))

    def get_super_block_summary(self):
        res = ""
        res += "magic:      0x{0:04x} ({1})\n".format(self.get_magic(), "VALID" if self.get_magic() == S5_MAGIC else "INVALID")
        res += "version:    0x{0:04x}{1}\n".format(self.get_version(), "" if self.get_version() == S5_CURRENT_VERSION else " (INVALID)")
        res += "num inodes: {0}\n".format(self.get_num_inodes())
        res += "free inode: {0}{1}\n".format(self.get_free_inode(), "" if self.get_free_inode() < self.get_num_inodes() else " (INVALID)")
        res += "root inode: {0}{1}\n".format(self.get_root_inode(), "" if self.get_root_inode() < self.get_num_inodes() else " (INVALID)")
        res += "free blocks ({0}{1}):\n".format(self.get_nfree(), "" if self.get_nfree() <= S5_NBLKS_PER_FNODE else (", too large shouldn't exceed " + str(S5_NBLKS_PER_FNODE)))
        for i in xrange(min(self.get_nfree(), S5_NBLKS_PER_FNODE - 1)):
            res += "  {0}".format(self.get_free_block(i))
            if ((i + 1) % 10 == 0):
                res += "\n"
        if (res[-1] != "\n"):
            res += "\n"
        res += "  last free block: {0}\n".format(self.get_last_free_block())
        return res

    def format(self, inodes, size):
        if (inodes < 1):
            raise S5fsException("cannot format disk with {0} inodes, must have at least one".format(inodes))
        if (size % S5_BLOCK_SIZE != 0):
            raise S5fsException("cannot format disk to size {0} which is not a multiple of the block size {1}".format(size, S5_BLOCK_SIZE))
        blocks = int(size / S5_BLOCK_SIZE)
        iblocks = int(math.floor((inodes - 1) / S5_INODES_PER_BLOCK) + 1)
        if (iblocks + 1 >= blocks):
            raise S5fsException("cannot format disk of size {0} with {1} inodes, the inodes require at least {2} bytes of space".format(size, inodes, (1 + iblocks) * S5_BLOCK_SIZE))
        self._simfile.truncate()
        self._simfile.seek(size)
        self._simfile.write("")

        self.set_magic(S5_MAGIC)
        self.set_version(S5_CURRENT_VERSION)
        self.set_num_inodes(inodes)
        for i in xrange(inodes):
            inode = self.get_inode(i)
            inode.set_number(i)
            inode.set_type(S5_TYPE_FREE)
            inode.set_next_free(i + 1)
        inode.set_next_free(0xffffffff)
        self.set_free_inode(0)

        self.set_last_free_block(0xffffffff)
        i = 0
        for num in xrange(iblocks+1, blocks):
            if (i == S5_NBLKS_PER_FNODE - 1):
                block = self.get_block(num)
                for j in xrange(S5_NBLKS_PER_FNODE - 1):
                    block.write(j * 4, struct.pack("I", self.get_free_block(j)))
                block.write((S5_NBLKS_PER_FNODE - 1) * 4, struct.pack("I", self.get_last_free_block()))
                self.set_last_free_block(num)
                i = 0
            else:
                self.set_free_block(i, num)
                i += 1
        self.set_nfree(i)

        root = self.alloc_inode()
        for i in xrange(S5_NDIRECT_BLOCKS):
            root.set_direct_blockno(i, 0)
        root.set_indirect_blockno(0)
        root.set_type(S5_TYPE_DIR)
        root.set_size(0)
        root.set_link_count(1)
        root._make_dirent(root.get_number(), ".")
        root._make_dirent(root.get_number(), "..")
        root.set_link_count(1)

    def free_inodes(self):
        inext = self.get_free_inode()
        while (inext != 0xffffffff):
            try:
                curr = self.get_inode(inext)
                yield inext
                inext = curr.get_free_inode()
            except S5fsException as e:
                raise S5fsException("error encountered while iterating free inodes: {0}".format(str(e)))

    def get_inode(self, index):
        offset = S5_BLOCK_SIZE * (1 + math.floor(index / S5_INODES_PER_BLOCK)) + S5_INODE_SIZE * (index % S5_INODES_PER_BLOCK)
        if (index >= self.get_num_inodes()):
            raise S5fsException("cannot get inode {0}, there are only {1} inodes on disk".format(index, self.get_num_inodes()))
        return Inode(self, index, offset)

    def alloc_inode(self):
        if (self.get_free_inode() == 0xffffffff):
            raise S5fsException("disk is out of inodes")
        inode = self.get_inode(self.get_free_inode())
        self.set_free_inode(inode.get_next_free())
        return inode

    def get_block(self, index):
        offset = S5_BLOCK_SIZE * index
        return Block(self, offset, index)

    def alloc_block(self):
        if (self.get_nfree() > S5_NBLKS_PER_FNODE - 1):
            raise S5fsException("nfree {0} is invalid, maximum value is {1}".format(self.get_nfree(), S5_NBLKS_PER_FNODE - 1))
        if (self.get_nfree() == 0):
            if (self.get_last_free_block() == 0xffffffff):
                raise S5fsDiskSpaceException()
            else:
                block = self.get_block(self.get_last_free_block())
                for i in xrange(S5_NBLKS_PER_FNODE - 1):
                    self.set_free_block(i, struct.unpack("I", block.read(i * 4, 4))[0])
                self.set_last_free_block(struct.unpack("I", block.read((S5_NBLKS_PER_FNODE - 1) * 4, 4))[0])
                self.set_nfree(S5_NBLKS_PER_FNODE - 1)
            return block
        else:
            self.set_nfree(self.get_nfree() - 1)
            return self.get_block(self.get_free_block(self.get_nfree()))

    def open(self, path, create=False):
        return self.get_inode(self.get_root_inode()).open(path, create=create)
