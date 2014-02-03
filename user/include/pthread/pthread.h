#pragma once

struct pthread;
struct pthread_cond;
struct pthread_mutex;

typedef struct pthread          *pthread_t;
typedef struct pthread_mutex    *pthread_mutex_t;
typedef struct pthread_cond     *pthread_cond_t;

/* Attributes NYI */
typedef int pthread_attr_t;
typedef int pthread_mutexattr_t;
typedef int pthread_condattr_t;

void            pthread_cleanup_pop(int);
void            pthread_cleanup_push(void (*)(void *), void *routine_arg);

int             pthread_cond_broadcast(pthread_cond_t *cond);
int             pthread_cond_destroy(pthread_cond_t *cond);
int             pthread_cond_init(pthread_cond_t *cond,
                                  const pthread_condattr_t *);
int             pthread_cond_signal(pthread_cond_t *cond);
int             pthread_cond_wait(pthread_cond_t *cond, pthread_mutex_t *mtx);
int             pthread_create(pthread_t *thr, const pthread_attr_t *,
                               void * ( *)(void *), void *);
int             pthread_detach(pthread_t thr);
int             pthread_equal(pthread_t, pthread_t);
void            pthread_exit(void *retval);
int             pthread_join(pthread_t thr, void **retval);
int             pthread_mutex_init(pthread_mutex_t *mtx,
                                   const pthread_mutexattr_t *);
int             pthread_mutex_lock(pthread_mutex_t *mtx);
int             pthread_mutex_trylock(pthread_mutex_t *mtx);
int             pthread_mutex_unlock(pthread_mutex_t *mtx);
void            pthread_yield(void);
int             pthread_cancel(pthread_t thr);

/* Everything below NYI */
#if 0
int             pthread_kill(pthread_t thr, int);
int             pthread_setcancelstate(int, int *);
int             pthread_setcanceltype(int, int *);
void            pthread_testcancel(void);
int             pthread_once(pthread_once_t *, void ( *)(void));
int             pthread_cond_timedwait(pthread_cond_t *,
                                       pthread_mutex_t *, const struct timespec *);
void            *pthread_getspecific(pthread_key_t);
int             pthread_key_create(pthread_key_t *,
                                   void ( *)(void *));
int             pthread_key_delete(pthread_key_t);
int             pthread_atfork(void ( *)(void), void ( *)(void), void ( *)(void));
int             pthread_attr_destroy(pthread_attr_t *);
int             pthread_attr_getstack(const pthread_attr_t *,
                                      void **, size_t *);
int             pthread_mutexattr_init(pthread_mutexattr_t *);
int             pthread_mutexattr_destroy(pthread_mutexattr_t *);
int             pthread_mutexattr_gettype(pthread_mutexattr_t *, int *);
int             pthread_mutexattr_settype(pthread_mutexattr_t *, int);
int             pthread_mutex_destroy(pthread_mutex_t *);
int             pthread_attr_getstacksize(const pthread_attr_t *, size_t *);
int             pthread_attr_getstackaddr(const pthread_attr_t *, void **);
int             pthread_attr_getguardsize(const pthread_attr_t *, size_t *);
int             pthread_attr_getdetachstate(const pthread_attr_t *, int *);
int             pthread_attr_init(pthread_attr_t *);
int             pthread_attr_setstacksize(pthread_attr_t *, size_t);
int             pthread_attr_setstack(pthread_attr_t *, void *, size_t);
int             pthread_attr_setstackaddr(pthread_attr_t *, void *);
int             pthread_attr_setguardsize(pthread_attr_t *, size_t);
int             pthread_attr_setdetachstate(pthread_attr_t *, int);
int             pthread_condattr_destroy(pthread_condattr_t *);
int             pthread_condattr_init(pthread_condattr_t *);
int             pthread_rwlock_destroy(pthread_rwlock_t *);
int             pthread_rwlock_init(pthread_rwlock_t *,
                                    const pthread_rwlockattr_t *);
int             pthread_rwlock_rdlock(pthread_rwlock_t *);
int             pthread_rwlock_timedrdlock(pthread_rwlock_t *,
                const struct timespec *);
int             pthread_rwlock_timedwrlock(pthread_rwlock_t *,
                const struct timespec *);
int             pthread_rwlock_tryrdlock(pthread_rwlock_t *);
int             pthread_rwlock_trywrlock(pthread_rwlock_t *);
int             pthread_rwlock_unlock(pthread_rwlock_t *);
int             pthread_rwlock_wrlock(pthread_rwlock_t *);
int             pthread_rwlockattr_init(pthread_rwlockattr_t *);
int             pthread_rwlockattr_getpshared(const pthread_rwlockattr_t *,
                int *);
int             pthread_rwlockattr_setpshared(pthread_rwlockattr_t *, int);
int             pthread_rwlockattr_destroy(pthread_rwlockattr_t *);
pthread_t       pthread_self(void);
int             pthread_setspecific(pthread_key_t, const void *);
int             pthread_sigmask(int, const sigset_t *, sigset_t *);

int             pthread_getprio(pthread_t);
int             pthread_setprio(pthread_t, int);

int             pthread_mutexattr_getprioceiling(pthread_mutexattr_t *,
                int *);
int             pthread_mutexattr_setprioceiling(pthread_mutexattr_t *,
                int);
int             pthread_mutex_getprioceiling(pthread_mutex_t *, int *);
int             pthread_mutex_setprioceiling(pthread_mutex_t *, int, int *);

int             pthread_mutexattr_getprotocol(pthread_mutexattr_t *, int *);
int             pthread_mutexattr_setprotocol(pthread_mutexattr_t *, int);

int             pthread_attr_getinheritsched(const pthread_attr_t *, int *);
int             pthread_attr_getschedparam(const pthread_attr_t *,
                struct sched_param *);
int             pthread_attr_getschedpolicy(const pthread_attr_t *, int *);
int             pthread_attr_getscope(const pthread_attr_t *, int *);
int             pthread_attr_setinheritsched(pthread_attr_t *, int);
int             pthread_attr_setschedparam(pthread_attr_t *,
                const struct sched_param *);
int             pthread_attr_setschedpolicy(pthread_attr_t *, int);
int             pthread_attr_setscope(pthread_attr_t *, int);
int             pthread_getschedparam(pthread_t pthread, int *,
                                      struct sched_param *);
int             pthread_setschedparam(pthread_t, int,
                                      const struct sched_param *);
int             pthread_getconcurrency(void);
int             pthread_setconcurrency(int);
#endif
