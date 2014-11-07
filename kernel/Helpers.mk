# A bunch of extra targets for stuff.

tidy:
	$(HIDE_SIGIL) rm -f $(BUILT_CRATES) $(OBJS) $(SYMBOLS) $(BSYMBOLS) $(KERNEL) $(IMAGE) $(ISO_IMAGE) $(GDBCOMM) */*.gdbcomm cscope*.out cscope.files
	$(HIDE_SIGIL) rm -rf .iso

clean: tidy $(foreach d,$(EXTERNAL_LIBS),clean-$(d))

cscope: $(HEAD) $(SRC)
	@ echo "  Updating cscope symbol cross-reference..."
	@ echo $(HEAD) $(SRC) > cscope.files
	@ $(CSCOPE) -k -b -q -v > /dev/null

FILTER=`echo "DRIVERS $(DRIVERS)\nVFS $(VFS)\nS5FS $(S5FS)\nVM $(VM)" | grep 1 | cut -f1 -d" " | tr "\n" "|"`PROCS
nyi:
	@ echo "  Not yet implemented:"
	@ echo
	@ find . -name \*.c -printf "%P\n" \
| xargs grep -Hn "NOT_YET_IMPLEMENTED" \
| sed -e 's/^\(.*:.*\):.*\"\(.*\): \(.*\)\".*/\2 \1 \3/' \
| grep -E "^($(FILTER))" \
| awk '{printf("%25s %30s() %8s\n", $$2, $$3, $$1)}'

todo:
	@ echo "  Not yet done:"
	@ echo
	@ git grep --heading --break -Epnh "((\/\/|\/?\*) TODO( [^C][^o][^p][^y][^r][^i][^g][^h][^t]|.{1,9}$$)|^ *\*t )" \
| sed -E "s/^\t?[0-9]+=/  /" \
| sed -E "/^  .*;$$/d" \
| awk '                                                       \
    BEGIN {                                                   \
        FS       = ":";                                       \
        count    = 0;                                         \
        cur_file = "";                                        \
        prev     = "";                                        \
        prev_cnt = 0;                                         \
    }                                                         \
    {                                                         \
        if ($$0 !~ /^[0-9]+:/) {                              \
            if ($$0 !~ /^ +/) {                               \
                if (cur_file != "") {                         \
                    plural = "";                              \
                    if (prev_cnt != 1) {                      \
                        plural = "s";                         \
                    }                                         \
                    printf("%s: %d thing%s to do\n%s\n",      \
                           cur_file, prev_cnt, plural, prev); \
                }                                             \
                cur_file = $$0;                               \
                prev_cnt = 0;                                 \
                prev     = "";                                \
            } else {                                          \
                prev = prev "" $$0 "\n";                      \
            }                                                 \
        } else {                                              \
            count    += 1;                                    \
            prev_cnt += 1;                                    \
            first     = $$1;                                  \
            $$1       = "";                                   \
            prev      = prev "" sprintf("%7s: %s\n",          \
                                        first, $$0);          \
        }                                                     \
    }                                                         \
    END {                                                     \
        if (cur_file != "") {                                 \
            plural = "";                                      \
            if (prev_cnt != 1) {                              \
                plural = "s";                                 \
            }                                                 \
            printf("%s: %d thing%s to do\n%s",                \
                   cur_file, prev_cnt, plural, prev);         \
        }                                                     \
        printf("\nTotal of %d TODOs\n", count);               \
    }                                                         \
    '
