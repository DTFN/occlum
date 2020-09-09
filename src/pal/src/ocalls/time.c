#include <stdio.h>
#include <pthread.h>
#include <sys/time.h>
#include "ocalls.h"

void occlum_ocall_gettimeofday(struct timeval *tv) {
    printf("occlum_ocall_gettimeofday\n");
    gettimeofday(tv, NULL);
}

void occlum_ocall_clock_gettime(int clockid, struct timespec *tp) {
    printf("occlum_ocall_clock_gettime\n");
    clock_gettime(clockid, tp);
}

void occlum_ocall_clock_getres(int clockid, struct timespec *res) {
    printf("occlum_ocall_clock_getres\n");
    clock_getres(clockid, res);
}

int occlum_ocall_nanosleep(const struct timespec *req, struct timespec *rem) {
    printf("occlum_ocall_nanosleep : %d\n",req->tv_sec);
    return nanosleep(req, rem);
}

int occlum_ocall_thread_getcpuclock(struct timespec *tp) {
    printf("occlum_ocall_thread_getcpuclock\n");
    clockid_t thread_clock_id;
    int ret = pthread_getcpuclockid(pthread_self(), &thread_clock_id);
    if (ret != 0) {
        PAL_ERROR("failed to get clock id");
        return -1;
    }

    return clock_gettime(thread_clock_id, tp);
}

void occlum_ocall_rdtsc(uint32_t *low, uint32_t *high) {
    printf("occlum_ocall_rdtsc\n");
    uint64_t rax, rdx;
    asm volatile("rdtsc" : "=a"(rax), "=d"(rdx));
    *low = (uint32_t)rax;
    *high = (uint32_t)rdx;
}
