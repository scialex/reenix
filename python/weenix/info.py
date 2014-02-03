import gdb
import weenix

_dbg_infofunc_type = gdb.lookup_type("dbg_infofunc_t")
_char_p_type = gdb.lookup_type("char").pointer()

def string(infofunc, data=None):
    weenix.assert_type("&" + infofunc, _dbg_infofunc_type)

    if (data == None):
        data = "0"

    npages = 8
    buf = weenix.eval_func("page_alloc_n", npages)

    weenix.eval_func(infofunc, data, buf, "PAGE_SIZE")
    res = buf.cast(_char_p_type).string()

    weenix.eval_func("page_free_n", buf, npages);
    return res
