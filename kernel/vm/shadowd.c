#include "types.h"
#include "globals.h"

#include "mm/mmobj.h"
#include "mm/pframe.h"

#include "util/debug.h"
#include "util/string.h"

#include "proc/proc.h"
#include "proc/sched.h"
#include "proc/kthread.h"

#ifdef __SHADOWD__
static ktqueue_t shadowd_waitq, kmem_alloc_waitq;
static int shadowd_initialized = 0;

void
shadowd_wakeup()
{
        /* If we run out of memory and need to wake up shadowd
         * before it has been properly initialized then the system
         * does not have enough memory. */
        KASSERT(shadowd_initialized);
        sched_broadcast_on(&shadowd_waitq);
}

void
shadowd_alloc_sleep()
{
        /* If we run out of memory and need to wake up shadowd
         * before it has been properly initialized then the system
         * does not have enough memory. */
        KASSERT(shadowd_initialized);
        sched_sleep_on(&kmem_alloc_waitq);
}

/*
 * The shadow daemon main routine. This should periodically
 * traverese all the shadow object trees, removing any
 * unnecessary shadow objects.
 *
 * A shadow object is considered unnecessary if it is not top most
 * (directly descendant from a vmarea), and if it has only 1
 * parent.
 *
 * For each shadow object we want to migrate all of its pages up
 * to the closest mmobj with at least 2 parents, or the topmost
 * one, then remove this object from the tree (if we remove it any
 * earlier we can cause big problems).
 *
 */

static void *
shadowd(int arg1, void *arg2)
{
        while (1) {
                proc_t *p;
                /* for each process, go through its vmareas */
                list_iterate_begin(proc_list(), p, proc_t, p_list_link) {
                        /* all of the dead process's shadow objects will be takenen care of by init */
                        if (PROC_RUNNING == p->p_state) {
                                vmarea_t *vma;
                                list_iterate_begin(&p->p_vmmap->vmm_list, vma, vmarea_t, vma_plink) {
                                        mmobj_t *last = vma->vma_obj, *o = last->mmo_shadowed;
                                        /* ref last, so if all processes on this branch die while shadowd is
                                         * sleeping, the branch won't get destroyed until shadowd() is done
                                         * with it */
                                        last->mmo_ops->ref(last);
                                        while (NULL != o && NULL != o->mmo_shadowed) {
                                                mmobj_t *shadow = o->mmo_shadowed;
                                                /* iff the object has only one parent, and is not right under vm_area */
                                                KASSERT(o != last);
                                                if (o->mmo_refcount - o->mmo_nrespages == 1) {
                                                        /* migrate all its pages to last, and remove it from the shadow tree */
                                                        pframe_t *pf;
                                                        list_iterate_begin(&o->mmo_respages, pf, pframe_t, pf_olink) {
                                                                /* Because the operations that could be
                                                                 * performed with an intermediate shadow object
                                                                 * to make pages busy are non-blocking,
                                                                 * we always expect to see non-busy pages. */
                                                                KASSERT(!pframe_is_busy(pf));
                                                                /* o has refcount 1+nrespages, so this won't delete it yet */
                                                                pframe_migrate(pf, last);
                                                        } list_iterate_end();
                                                        last->mmo_shadowed = o->mmo_shadowed;
                                                        /* Ref o's shadowed, so we don't accidentally delete it when we
                                                         * finally put o */
                                                        o->mmo_shadowed->mmo_ops->ref(o->mmo_shadowed);
                                                        KASSERT(o->mmo_refcount == 1 && o->mmo_nrespages == 0);
                                                        o->mmo_ops->put(o);
                                                } else {
                                                        KASSERT(o->mmo_refcount - o->mmo_nrespages == 2);
                                                        o->mmo_ops->ref(o);
                                                        last->mmo_ops->put(last);
                                                        last = o;
                                                }
                                                o = shadow;
                                        }
                                        KASSERT(NULL != last);
                                        last->mmo_ops->put(last);
                                } list_iterate_end();
                        }
                } list_iterate_end();

                sched_broadcast_on(&kmem_alloc_waitq);
                if (sched_cancellable_sleep_on(&shadowd_waitq) < 0) {
                        return (void *)0;
                }
        }
}

static proc_t *shadowd_proc;
static kthread_t *shadowd_thr;

static __attribute__((unused)) void
shadowd_init()
{
        sched_queue_init(&shadowd_waitq);
        sched_queue_init(&kmem_alloc_waitq);

        KASSERT(NULL != curproc && (PID_IDLE == curproc->p_pid));
        shadowd_proc = proc_create("shadowd");
        KASSERT(NULL != shadowd_proc);
        shadowd_thr = kthread_create(shadowd_proc, shadowd, 0, NULL);
        KASSERT(NULL != shadowd_thr);

        sched_make_runnable(shadowd_thr);

        shadowd_initialized = 1;
}
init_func(shadowd_init);
init_depends(sched_init);

/*
 * Cancel the shadowd
 */
void
shadowd_shutdown()
{
        KASSERT(NULL != shadowd_thr);
        KASSERT(PID_IDLE == curproc->p_pid);
        kthread_cancel(shadowd_thr, (void *)0);
        shadowd_thr = NULL;
        int shadow_pid = shadowd_proc->p_pid;
        int child = do_waitpid(-1, 0, NULL);
        KASSERT(child == shadow_pid && "waited on process other than pageoutd");
}
#endif
